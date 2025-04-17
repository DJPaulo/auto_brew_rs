#![no_std]
#![no_main]

use defmt::*;

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

use auto_brew_rs::{display::*, sensor::Ds18b20};

// static variables
static LAST_UPDATE: StaticCell<Instant> = StaticCell::new(); // The last time the temp was checked
static LAST_DISPLAY: StaticCell<Instant> = StaticCell::new(); // The last time the display was updated
// static PIN_INTERRUPT: bool = false;         // Indicates that there was an interrupt from a GPIO pin
// static DISPLAY_KEY0_PRESSED: bool = false;  // Indicates if button (key0) was pressed
// static DISPLAY_KEY1_PRESSED: bool = false;  // Indicates if button (key1) was pressed
static CURRENT_TEMP: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);             // The current temperature reading
// static NO_DEVICE: bool = false;             // Indicates if no temperature sensor was detected
// static DISPLAY_ON: bool = true;             // Indicates that the display is on
// static RELAY_ON: bool = false;              // Indicates that a relay is on
// static SWITCH_OFF_RELAYS: bool = false;     // Indicates if relays should be switched off
//static TARGET_TEMP: f64 = 19.0;             // Target temperature to maintain (Default = 19 degrees C)
// static INTEGRAL: f64 = 0.0;                 // The calculated integral value
// static LAST_VARIANCE: f64 = 0.0;            // The last calculated variance

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

async fn initialise_variables() {
    LAST_UPDATE.init(Instant::now());
    LAST_DISPLAY.init(Instant::now());
}


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;
    //let del = Delay;

    // Set the initial values for the timers
    initialise_variables().await;
    

    // Thermometer pins
    let mut pio = Pio::new(peripherals.PIO0, Irqs);

    let prg = PioOneWireProgram::new(&mut pio.common);
    let onewire = PioOneWire::new(&mut pio.common, pio.sm0, peripherals.PIN_16, &prg);


    // Set up thermometer
    let mut temp_sensor = Ds18b20::new(onewire);

    temp_sensor.start().await; // Start a new measurement
    Timer::after_secs(1).await; // Allow 1s for the measurement to finish
    match temp_sensor.temperature().await {
        Ok(temp) => {
            *CURRENT_TEMP.lock().await = temp;
            info!("temp = {:?} deg C", *CURRENT_TEMP.lock().await);
        },
        _ => error!("sensor error"),
    }
    Timer::after_secs(1).await;

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

    
    let _ = display.refresh_line_1("stuff").await;
    let _ = display.refresh_line_2("stuffs").await;
    let _ = display.refresh_line_3("stuffses").await;
    let _ = display.refresh_line_4("This are message").await;
    let _ = display.show().await;

    delay.delay_ms(10000).await;
    


    
//    delay.delay_ms(4000).await;
    
    //    info!("Begin loop logic");
    //    loop {

    //    }

    info!("Program end");
}
