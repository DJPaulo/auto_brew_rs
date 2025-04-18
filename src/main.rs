#![no_std]
#![no_main]
use core::fmt::Write;
use defmt::{error, info};
use heapless::String;
use embedded_hal_async::delay::DelayNs;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
//use embassy_rp::interrupt;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::onewire::{PioOneWire, PioOneWireProgram};
use embassy_time::{Delay, Instant, Timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

use auto_brew_rs::{display::*, sensor::*, AutoBrewError};

// static variables

static NO_DEVICE: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);              // Indicates if no temperature sensor was detected
static LAST_UPDATE: StaticCell<Instant> = StaticCell::new();                        // The last time the temp was checked
static LAST_DISPLAY: StaticCell<Instant> = StaticCell::new();                       // The last time the display was updated
static CURRENT_TEMP: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);              // The current temperature reading
static TARGET_TEMP: Mutex<ThreadModeRawMutex, f32> = Mutex::new(19.0);              // Target temperature to maintain (Default = 19 degrees C)
static CURRENT_VARIANCE: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);          // The current variance
static LAST_VARIANCE: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);             // The last calculated variance
static PIN_INTERRUPT: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);          // Indicates that there was an interrupt from a GPIO pin
static DISPLAY_KEY0_PRESSED: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);   // Indicates if button (key0) was pressed
static DISPLAY_KEY1_PRESSED: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);   // Indicates if button (key1) was pressed


// static DISPLAY_ON: bool = true;             // Indicates that the display is on
// static RELAY_ON: bool = false;              // Indicates that a relay is on
// static SWITCH_OFF_RELAYS: bool = false;     // Indicates if relays should be switched off

// static INTEGRAL: f32 = 0.0;                 // The calculated integral value


// constants
// const OFF: i8 = 0;                          // Value for OFF
// const ON: i8 = 1;                           // Value for ON
// const MIN_TEMP: f32 = 11.0;                 // Minimum selectable temp
// const MAX_TEMP: f32 = 27.0;                 // Maximum selectable temp
// const CHECK_IN: i16 = 300;                   // Temperature check interval (seconds)
// const NO_DEVICE_CHECK_IN: i8 = 60;          // Check interval for when no temperature sensor was detected previously (seconds)
// const DISPLAY_TIMEOUT: i8 = 30;             // Turn off display to avoid burn-in
// const TOLERANCE: f32 = 0.25;                // Allowable variance on either side of the target
// const KP: f32 = 10.0;                       // Proportional term - Basic steering (This is the first parameter you should tune for a particular setup)
// const KI: f32 = 0.01;                       // Integral term - Compensate for heat loss by vessel
// const KD: f32 = 150.0;                      // 


//#[cortex_m_rt::pre_init]
//unsafe fn before_main() {
// Soft-reset doesn't clear spinlocks. Clear the one used by critical-section
// before we hit main to avoid deadlocks when using a debugger
//    embassy_rp::pac::SIO.spinlock(31).write_value(1);
//}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

async fn initialise_times() {
    LAST_UPDATE.init(Instant::now());
    LAST_DISPLAY.init(Instant::now());
}

async fn get_current_temp(mut temp_sensor: Ds18b20<'_, PIO0, 0>) -> Result<(f32), AutoBrewError> {
    temp_sensor.start().await; // Start a new measurement
    Timer::after_secs(1).await; // Allow 1s for the measurement to finish
    match temp_sensor.temperature().await {
        Ok(temp) => {
            *NO_DEVICE.lock().await = false;
            *CURRENT_TEMP.lock().await = temp;
            *CURRENT_VARIANCE.lock().await = *TARGET_TEMP.lock().await - *CURRENT_TEMP.lock().await;
            info!("temp = {:?} deg C", temp);   // Debug colsole
            Ok((temp))
        },
        _ => {
            *NO_DEVICE.lock().await = true;
            error!("Sensor not found");     // Debug console
            Err(AutoBrewError::SensorNotFoundError)
        }
    }
}

// Convert a f32 value into a string
fn f32_to_string(value: f32) -> String<16> {
    let mut string: String<16> = String::new();
    let _ = write!(&mut string, "{:.1}", value);
    string
}


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;

 
    // Thermometer pins
    let mut pio = Pio::new(peripherals.PIO0, Irqs);
    // Set up onewire
    let prg = PioOneWireProgram::new(&mut pio.common);
    let onewire = PioOneWire::new(&mut pio.common, pio.sm0, peripherals.PIN_16, &prg);
    // Set up thermometer
    let mut temp_sensor = Ds18b20::new(onewire);
    let _ = temp_sensor.set_resolution(Resolution::Bits12).await; // Set the resolution to 12 bits (0.0625 degrees C)



    // Initialise the display and show the splash screen
    let display_peripherals = DisplayPeripherals::new(
        Output::new(peripherals.PIN_8, Level::Low),  // Data/Command
        Output::new(peripherals.PIN_9, Level::High), // Chip Select
        Output::new(peripherals.PIN_12, Level::Low), // Reset
        peripherals.PIN_10,     // Serial Clock
        peripherals.PIN_11,     // Master Out Slave In
        peripherals.SPI1,       // SPI peripheral
        peripherals.DMA_CH0,    // DMA channel
    );
    let mut display = Display::new(display_peripherals);
    let _ = display.initialise().await;
    delay.delay_ms(10).await;
    let _ = display.show_splash_screen().await;


    let _ = get_current_temp(temp_sensor).await;    // Get a temperature reading
    let cur_tmp = f32_to_string(*CURRENT_TEMP.lock().await);
    let tar_tmp = f32_to_string(*TARGET_TEMP.lock().await);
    let cur_var = f32_to_string(*CURRENT_VARIANCE.lock().await);
    let mut msg = "";
    if *NO_DEVICE.lock().await == true {
        msg = "Sensor not found";
    }
    let _ = display.refresh_readings(cur_tmp.as_str(), tar_tmp.as_str(), cur_var.as_str(), msg).await;
    delay.delay_ms(5000).await; // ** NB ** Remove after testing

    initialise_times().await;   // Set the initial values for the timers
    
    // Main loop
    info!("Begin loop logic");
    loop {
        // Check if a button was pressed
        if *PIN_INTERRUPT.lock().await == true {
            

        }
        else {
            


        }


    }

    //info!("Program end");
}
