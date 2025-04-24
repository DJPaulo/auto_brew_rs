#![no_std]
#![no_main]
use core::fmt::Write;
use defmt::{error, info};
use heapless::String;
use embedded_hal_async::delay::DelayNs;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output, Input, Pull};
use embassy_rp::peripherals::{PIO0, FLASH};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::onewire::{PioOneWire, PioOneWireProgram};
use embassy_time::{Delay, Instant, Timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use {defmt_rtt as _, panic_probe as _};
use auto_brew_rs::{display::*, sensor::*, AutoBrewError};

// static variables
static NO_DEVICE: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);              // Indicates if no temperature sensor was detected
static LAST_UPDATE: Mutex<ThreadModeRawMutex, u64> = Mutex::new(0);                 // The last time the temp was checked
static LAST_DISPLAY: Mutex<ThreadModeRawMutex, u64> = Mutex::new(0);                // The last time the display was updated
static CURRENT_TEMP: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);              // The current temperature reading
static TARGET_TEMP: Mutex<ThreadModeRawMutex, f32> = Mutex::new(19.0);              // Target temperature to maintain (Default = 19 degrees C)
static CURRENT_VARIANCE: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);          // The current variance
static LAST_VARIANCE: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);             // The last calculated variance
static PIN_INTERRUPT: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);          // Indicates that there was an interrupt from a GPIO pin
static DISPLAY_KEY0_PRESSED: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);   // Indicates if button (key0) was pressed
static DISPLAY_KEY1_PRESSED: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);   // Indicates if button (key1) was pressed
static DISPLAY_ON: Mutex<ThreadModeRawMutex, bool> = Mutex::new(true);              // Indicates that the display is on
static RELAY_ON: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);               // Indicates that a relay is on
static SWITCH_OFF_RELAYS: Mutex<ThreadModeRawMutex, u64> = Mutex::new(0);           // The time that the relays should be switched off at
static INTEGRAL: Mutex<ThreadModeRawMutex, f32> = Mutex::new(0.0);                  // The calculated integral value

// constants
const MIN_TEMP: f32 = 11.0;                 // Minimum selectable temp
const MAX_TEMP: f32 = 27.0;                 // Maximum selectable temp
const CHECK_IN: i16 = 300;                  // Temperature check interval (seconds)
const NO_DEVICE_CHECK_IN: i8 = 60;          // Check interval for when no temperature sensor was detected previously (seconds)
const DISPLAY_TIMEOUT: i8 = 30;             // Turn off display to avoid burn-in
const TOLERANCE: f32 = 0.25;                // Allowable variance on either side of the target
const KP: f32 = 10.0;                       // Proportional term - Basic steering (This is the first parameter you should tune for a particular setup)
const KI: f32 = 0.01;                       // Integral term - Compensate for heat loss by vessel
const KD: f32 = 150.0;                      // Derivative term - Compensate for overshoot (This is the last parameter you should tune for a particular setup)

const FLASH_SIZE: usize = 2 * 1024 * 1024;  // 2MB flash
const ADDR_OFFSET: u32 = 0x100000;  // Start at 1MB offset

//#[cortex_m_rt::pre_init]
//unsafe fn before_main() {
// Soft-reset doesn't clear spinlocks. Clear the one used by critical-section
// before we hit main to avoid deadlocks when using a debugger
//    embassy_rp::pac::SIO.spinlock(31).write_value(1);
//}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// Get the current temperature from the sensor and update the global variables
async fn get_current_temp(temp_sensor: &mut Ds18b20<'_, PIO0, 0>) -> Result<f32, AutoBrewError> {
    temp_sensor.start().await;      // Start a new measurement
    Timer::after_secs(1).await;     // Allow 1s for the measurement to finish
    match temp_sensor.temperature().await {
        Ok(temp) => {
            *NO_DEVICE.lock().await = false;
            *CURRENT_TEMP.lock().await = temp;
            *CURRENT_VARIANCE.lock().await = *TARGET_TEMP.lock().await - *CURRENT_TEMP.lock().await;
            info!("temp = {:?} deg C", temp);   // Debug colsole
            Ok(temp)
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

// Round to the nearest integer value
fn round(x: f32) -> i32 {
    if x >= 0.0 {
        (x + 0.5) as i32
    } else {
        (x - 0.5) as i32
    }
}

// Save the target temperature to flash memory
async fn save_target_temp(flash: &mut Flash<'_, FLASH, Async, FLASH_SIZE>, temp: f32) {
    let bytes = temp.to_le_bytes();
    flash.blocking_erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32).unwrap();
    flash.blocking_write(ADDR_OFFSET, &bytes).unwrap();
}

// Load the target temperature from flash memory
async fn load_target_temp(flash: &mut Flash<'_, FLASH, Async, FLASH_SIZE>) -> Option<f32> {
    let mut bytes = [0u8; 4];
    flash.read(ADDR_OFFSET, &mut bytes).await.unwrap();
    let temp = f32::from_le_bytes(bytes);
    if temp >= MIN_TEMP && temp <= MAX_TEMP {
        Some(temp)
    } else {
        None
    }
}


#[embassy_executor::task]
async fn gpio_task(mut key0: Input<'static>, mut key1: Input<'static>) {
    loop {
        // Create futures for both buttons
        let key0_future = key0.wait_for_falling_edge();
        let key1_future = key1.wait_for_falling_edge();

        if *DISPLAY_ON.lock().await {
            // Wait for either button to be pressed
            match embassy_futures::select::select(key0_future, key1_future).await {
                // Key0 was pressed
                embassy_futures::select::Either::First(_) => {
                    *PIN_INTERRUPT.lock().await = true;
                    *DISPLAY_KEY0_PRESSED.lock().await = true;
                    info!("Key0 pressed");      // Debug colsole
                    while key0.is_low() {
                        Timer::after_millis(10).await;  // Wait for the button to be released
                    }
                }
                // Key1 was pressed
                embassy_futures::select::Either::Second(_) => {
                    *PIN_INTERRUPT.lock().await = true;
                    *DISPLAY_KEY1_PRESSED.lock().await = true;
                    info!("Key1 pressed");      // Debug colsole
                    while key1.is_low() {
                        Timer::after_millis(10).await;  // Wait for the button to be released
                    }
                }
            }
        }
        *LAST_DISPLAY.lock().await = Instant::now().as_secs();    // Update last_display time
        Timer::after_millis(100).await;  // Debounce delay
    }
}


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");     // Debug colsole
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;

    // Read from flash memory if the target temperature has been set previously
    let mut flash = Flash::<_, Async, FLASH_SIZE>::new(peripherals.FLASH, peripherals.DMA_CH1);
    if let Some(saved_temp) = load_target_temp(&mut flash).await {
        *TARGET_TEMP.lock().await = saved_temp;
    }
 
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

    // Set up the display's input button pins with pull-up resistors
    let display_key0 = Input::new(peripherals.PIN_15, Pull::Up);
    let display_key1 = Input::new(peripherals.PIN_17, Pull::Up);

    // Spawn the GPIO task to handle interrupts
    _spawner.spawn(gpio_task(display_key0, display_key1)).unwrap();

    let _ = get_current_temp(&mut temp_sensor).await;    // Get a temperature reading
    let mut msg = "";
    if *NO_DEVICE.lock().await {
        msg = "SENSOR NOT FOUND";
        let _ = display.clear_all().await;
        let _ = display.refresh_line_4(msg).await;
        let _ = display.show().await;
    }
    else {
        let cur_tmp = f32_to_string(*CURRENT_TEMP.lock().await);
        let tar_tmp = f32_to_string(*TARGET_TEMP.lock().await);
        let cur_var = f32_to_string(*CURRENT_VARIANCE.lock().await);
        let _ = display.refresh_readings(cur_tmp.as_str(), tar_tmp.as_str(), cur_var.as_str(), msg).await;
    }
    
    //delay.delay_ms(5000).await; // ** NB ** Remove after testing

    // Set up the GPIO pins for the heating and cooling relays
    let mut heating_relay = Output::new(peripherals.PIN_6, Level::Low); // Relay 1 for heating
    let mut cooling_relay = Output::new(peripherals.PIN_7, Level::Low); // Relay 2 for cooling

    // Main loop
    info!("Begin loop logic");      // Debug colsole
    loop {
        // Check if a button was pressed
        if *PIN_INTERRUPT.lock().await {
            if *DISPLAY_KEY0_PRESSED.lock().await && *NO_DEVICE.lock().await == false {
                if *TARGET_TEMP.lock().await < MAX_TEMP && *DISPLAY_ON.lock().await {
                    *TARGET_TEMP.lock().await += 0.5;
                    *CURRENT_VARIANCE.lock().await = *TARGET_TEMP.lock().await - *CURRENT_TEMP.lock().await; // Update the variance
                    save_target_temp(&mut flash, *TARGET_TEMP.lock().await).await;  // Save new temp
                }
                *DISPLAY_KEY0_PRESSED.lock().await = false;     // Turn off the key0 press flag after it has been handled
            }
            if *DISPLAY_KEY1_PRESSED.lock().await && *NO_DEVICE.lock().await == false {
                if *TARGET_TEMP.lock().await > MIN_TEMP && *DISPLAY_ON.lock().await {
                    *TARGET_TEMP.lock().await -= 0.5;
                    *CURRENT_VARIANCE.lock().await = *TARGET_TEMP.lock().await - *CURRENT_TEMP.lock().await; // Update the variance
                    save_target_temp(&mut flash, *TARGET_TEMP.lock().await).await;  // Save new temp
                }
                *DISPLAY_KEY1_PRESSED.lock().await = false;     // Turn off the key1 press flag after it has been handled
            }
            if *DISPLAY_ON.lock().await == false {
                *DISPLAY_ON.lock().await = true;    // To wake up the display if it was off
            }
            if *NO_DEVICE.lock().await {
                let _ = display.clear_all().await;
                let _ = display.refresh_line_4("SENSOR NOT FOUND").await;
                let _ = display.show().await;
            }
            else {
                let cur_tmp = f32_to_string(*CURRENT_TEMP.lock().await);
                let tar_tmp = f32_to_string(*TARGET_TEMP.lock().await);
                let cur_var = f32_to_string(*CURRENT_VARIANCE.lock().await);
                let mut msg = "";
                if heating_relay.is_set_high() { msg = "   HEATING ON   "; }
                if cooling_relay.is_set_high() { msg = "   COOLING ON   "; }
                let _ = display.refresh_readings(cur_tmp.as_str(), tar_tmp.as_str(), cur_var.as_str(), msg).await;
            }
            *PIN_INTERRUPT.lock().await = false;    // Turn off the interrupt flag after it has been handled
        }
        else {
            let now = Instant::now().as_secs();
            // Check how long the display has been on for
            if now >= *LAST_DISPLAY.lock().await + DISPLAY_TIMEOUT as u64 {
                *DISPLAY_ON.lock().await = false;    // Turn off the display
                let _ = display.clear_all().await;
                let _ = display.show().await;
            }
            
            // Set the check interval based on whether a device was detected
            let check_seconds: u64 = match *NO_DEVICE.lock().await {
                true => NO_DEVICE_CHECK_IN as u64,
                false => CHECK_IN as u64,
            };

            // Check if it is time to switch off the relays
            if *RELAY_ON.lock().await && now >= *SWITCH_OFF_RELAYS.lock().await {
                // Switch off the relays
                heating_relay.set_low();
                cooling_relay.set_low();
                info!("Relays off");     // Debug colsole
                *RELAY_ON.lock().await = false;
                // Clear the messsage line
                if *DISPLAY_ON.lock().await {
                    let _ =  display.clear_line_4().await;
                    let _ =  display.show().await;
                }
            }

            let time_diff = now - *LAST_UPDATE.lock().await;
            info!("Here");     // Debug colsole
            // Check if it is time to get a new temperature reading
            if time_diff > check_seconds {
                info!("getting new reading");     // Debug colsole
                let _ = get_current_temp(&mut temp_sensor).await;    // Get a temperature reading

                if *NO_DEVICE.lock().await {
                    if *DISPLAY_ON.lock().await {
                       let _ = display.clear_all().await;
                       let _ = display.refresh_line_4("Sensor not found").await;
                       let _ = display.show().await;
                    }
                }
                else {
                    // Display the latest readings
                    if *DISPLAY_ON.lock().await {
                        let cur_tmp = f32_to_string(*CURRENT_TEMP.lock().await);
                        let tar_tmp = f32_to_string(*TARGET_TEMP.lock().await);
                        let cur_var = f32_to_string(*CURRENT_VARIANCE.lock().await);
                        let msg = "";
                        let _ = display.refresh_readings(cur_tmp.as_str(), tar_tmp.as_str(), cur_var.as_str(), msg).await;
                    }
                    info!("Then here");     // Debug colsole
                    if (*CURRENT_VARIANCE.lock().await).abs() > TOLERANCE {
                        let integral = ((time_diff as f32) * *CURRENT_VARIANCE.lock().await) + *INTEGRAL.lock().await;
                        //*INTEGRAL.lock().await = *INTEGRAL.lock().await + time_diff as f32 * *CURRENT_VARIANCE.lock().await;
                        *INTEGRAL.lock().await = integral;
                        let derivative = (*CURRENT_VARIANCE.lock().await - *LAST_VARIANCE.lock().await) / time_diff as f32;
                        let output = KP * *CURRENT_VARIANCE.lock().await + KI * *INTEGRAL.lock().await + KD * derivative;
                        let out = round(output);

                        if out > 0 {
                            info!("Heating on");     // Debug colsole
                            if *DISPLAY_ON.lock().await {
                                let _ = display.refresh_line_4("   HEATING ON   ").await;
                                let _ = display.show().await;
                            }
                            *RELAY_ON.lock().await = true;
                            *SWITCH_OFF_RELAYS.lock().await = Instant::now().as_secs() + out.abs() as u64;
                            heating_relay.set_high();
                        }

                        if out < 0 {
                            info!("Cooling on");     // Debug colsole
                            if *DISPLAY_ON.lock().await {
                                let _ = display.refresh_line_4("   COOLING ON   ").await;
                                let _ = display.show().await;
                            }
                            *RELAY_ON.lock().await = true;
                            *SWITCH_OFF_RELAYS.lock().await = Instant::now().as_secs() + out.abs() as u64;
                            cooling_relay.set_high();
                        }

                    }
                    *LAST_VARIANCE.lock().await = *CURRENT_VARIANCE.lock().await;    // Update the last variance
                }
                *LAST_UPDATE.lock().await = now;    // Update the last update time
            }
            
             

        }
        delay.delay_ms(500).await;

    }

    //info!("Program end");
}
