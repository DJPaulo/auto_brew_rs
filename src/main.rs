#![no_std]
#![no_main]

use defmt::*;

use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{self, InterruptHandler, Pio};
use embassy_rp::pio_programs::onewire::{PioOneWire, PioOneWireProgram};
use embassy_time::{Delay, Duration, Instant, Timer};
//use embassy_rp::i2c::InterruptHandler;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};
//use embassy_sync::blocking_mutex::raw::NoopRawMutex;
//use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use display_interface::{AsyncWriteOnlyDataCommand, DisplayError};
use display_interface_spi::SPIInterface;

use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Text, TextStyleBuilder};

//use ds18b20;

//use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use auto_brew_rs::{adjustment, controls, sensor::Ds18b20, sh1107};

// static variables
static LAST_UPDATE: StaticCell<Instant> = StaticCell::new(); // The last time the temp was checked
static LAST_DISPLAY: StaticCell<Instant> = StaticCell::new(); // The last time the display was updated
static PIN_INTERRUPT: bool = false;         // Indicates that there was an interrupt from a GPIO pin
static DISPLAY_KEY0_PRESSED: bool = false;  // Indicates if button (key0) was pressed
static DISPLAY_KEY1_PRESSED: bool = false;  // Indicates if button (key1) was pressed
static CURRENT_TEMP: f32 = 0.0;             // The current temperature reading
static NO_DEVICE: bool = false;             // Indicates if no temperature sensor was detected
static DISPLAY_ON: bool = true;             // Indicates that the display is on
static RELAY_ON: bool = false;              // Indicates that a relay is on
static SWITCH_OFF_RELAYS: bool = false;     // Indicates if relays should be switched off
static TARGET_TEMP: f64 = 19.0;             // Target temperature to maintain (Default = 19 degrees C)
static INTEGRAL: f64 = 0.0;                 // The calculated integral value
static LAST_VARIANCE: f64 = 0.0;            // The last calculated variance

// constants
const OFF: i8 = 0;                          // Value for OFF
const ON: i8 = 1;                           // Value for ON
const MIN_TEMP: f32 = 11.0;                 // Minimum selectable temp
const MAX_TEMP: f32 = 27.0;                 // Maximum selectable temp
const CHECK_IN: i16 = 300;                   // Temperature check interval (seconds)
const NO_DEVICE_CHECK_IN: i8 = 60;          // Check interval for when no temperature sensor was detected previously (seconds)
const DISPLAY_TIMEOUT: i8 = 30;             // Turn off display to avoid burn-in
const TOLERANCE: f32 = 0.25;                // Allowable variance on either side of the target
const KP: f32 = 10.0;                       // Proportional term - Basic steering (This is the first parameter you should tune for a particular setup)
const KI: f32 = 0.01;                       // Integral term - Compensate for heat loss by vessel
const KD: f32 = 150.0;                      // 

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

/*
async fn reset_last_update_time() {
    let mut last_update_time = LAST_UPDATE.lock().await;
    *last_update_time = Some(Instant::now());
}

async fn reset_last_display_time() {
    let mut last_display_time = LAST_DISPLAY.lock().await;
    *last_display_time = Some(Instant::now());
}

async fn get_last_update_elapsed_time() -> Option<Duration> {
    let last_update_time = LAST_UPDATE.lock().await;
    if let Some(update) = *last_update_time {
        Some(Instant::now() - update)
    } else {
        None
    }
}

async fn get_last_display_elapsed_time() -> Option<Duration> {
    let last_display_time = LAST_DISPLAY.lock().await;
    if let Some(display) = *last_display_time {
        Some(Instant::now() - display)
    } else {
        None
    }
}


// Generic function to reset a given `Mutex<Option<Instant>>`
async fn reset_time(scell: &'static StaticCell<Option<Instant>>) {
    scell.init(Some(Instant::now()));
}

// Generic function to get the elapsed time for a given `Mutex<Option<Instant>>`
async fn get_elapsed_time(scell: &'static StaticCell<Option<Instant>>) -> Option<Duration> {
    if let Some(start) = scell.as_ref() {
        Some(Instant::now() - start)
    } else {
        None
    }
}
*/
// Reset the last update time
//reset_time(&LAST_UPDATE).await;

//if let Some(elapsed) = get_elapsed_time(&LAST_UPDATE).await {
//    println!("Elapsed time since last update: {:?}", elapsed);
//}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;
    let del = Delay;

    // Display pins
    let dc = Output::new(peripherals.PIN_8, Level::Low); // Data/Command
    let cs = Output::new(peripherals.PIN_9, Level::High); // Chip Select
    let sclk = peripherals.PIN_10; // Serial Clock
    let mosi = peripherals.PIN_11; // Master Out Slave In
    let rst = Output::new(peripherals.PIN_12, Level::Low); // Reset

    // Thermometer pins
    let mut pio = Pio::new(peripherals.PIO0, Irqs);

    let prg = PioOneWireProgram::new(&mut pio.common);
    let onewire = PioOneWire::new(&mut pio.common, pio.sm0, peripherals.PIN_16, &prg);

    initialise_times().await;

    let mut temp_sensor = Ds18b20::new(onewire);

    temp_sensor.start().await; // Start a new measurement
    Timer::after_secs(1).await; // Allow 1s for the measurement to finish
    match temp_sensor.temperature().await {
        Ok(temp) => {
            //CURRENT_TEMP = temp;
            info!("temp = {:?} deg C", temp)
        },
        _ => error!("sensor error"),
    }
    Timer::after_secs(1).await;

    let mut spi_config = Config::default();
    spi_config.frequency = 2_000_000;
    spi_config.phase = Phase::CaptureOnSecondTransition;
    spi_config.polarity = Polarity::IdleHigh;

    let spi = Spi::new_txonly(
        peripherals.SPI1,
        sclk,
        mosi,
        peripherals.DMA_CH0,
        spi_config,
    );
    let mut spi_device = ExclusiveDevice::new(spi, cs, del).unwrap();

    let mut display = sh1107::SH1107::new(&mut spi_device, dc, rst);

    // Set up thermometer
    //
    //

    let _ = display.init(&mut delay).await;
    delay.delay_ms(1000).await;

    let _ = display.clear().await;
    let _ = display.show().await;

    let _ = display
        .draw_rectangle(Point::new(0, 0), Size::new(128, 64), BinaryColor::On, false)
        .await;
    //display.show().await;
    //delay.delay_ms(4000).await;

    let _ = display
        .draw_text("   AutoBrew rs ", Point::new(0, 22), BinaryColor::On)
        .await;
    let _ = display
        .draw_text("     v0.1.0    ", Point::new(0, 40), BinaryColor::On)
        .await;
    delay.delay_ms(10).await;
    let _ = display.show().await;
    delay.delay_ms(5000).await;
    let _ = display.clear().await;
    let _ = display.show().await;
    delay.delay_ms(10).await;

    //    info!("Begin loop logic");
    //    loop {

    //    }

    info!("Program end");
}
