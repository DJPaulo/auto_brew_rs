#![no_std]
#![no_main]

use defmt::*;

use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, Delay};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{self, InterruptHandler, Pio};
use embassy_rp::pio_programs::onewire::{{PioOneWire, PioOneWireProgram}};
//use embassy_rp::i2c::InterruptHandler;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};


use display_interface::{DisplayError, AsyncWriteOnlyDataCommand };
use display_interface_spi::SPIInterface;

use embedded_graphics::prelude::*;
use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyle};
use embedded_graphics::text::{Text, TextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;

//use ds18b20;

//use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use auto_brew_rs::{adjustment, controls, sensor::Ds18b20, sh1107 as sh1107};


//#[cortex_m_rt::pre_init]
//unsafe fn before_main() {
    // Soft-reset doesn't clear spinlocks. Clear the one used by critical-section
    // before we hit main to avoid deadlocks when using a debugger
//    embassy_rp::pac::SIO.spinlock(31).write_value(1);
//}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;
    let mut del = Delay;

    // Display pins  
    let dc = Output::new(peripherals.PIN_8, Level::Low);     // Data/Command
    let cs = Output::new(peripherals.PIN_9, Level::High);    // Chip Select
    let sclk = peripherals.PIN_10;                               // Serial Clock
    let mosi = peripherals.PIN_11;                               // Master Out Slave In
    let rst = Output::new(peripherals.PIN_12, Level::Low);   // Reset

    // Thermometer pins
    let mut pio = Pio::new(peripherals.PIO0, Irqs);

    let prg = PioOneWireProgram::new(&mut pio.common);
    let onewire = PioOneWire::new(&mut pio.common, pio.sm0, peripherals.PIN_16, &prg);

    let mut temp_sensor = Ds18b20::new(onewire);


    temp_sensor.start().await; // Start a new measurement
    Timer::after_secs(1).await; // Allow 1s for the measurement to finish
    match temp_sensor.temperature().await {
        Ok(temp) => info!("temp = {:?} deg C", temp),
        _ => error!("sensor error"),
    }
    Timer::after_secs(1).await;


    let mut spi_config = Config::default();
    spi_config.frequency = 2_000_000;
    spi_config.phase = Phase::CaptureOnSecondTransition;
    spi_config.polarity = Polarity::IdleHigh;


    let spi = Spi::new_txonly(peripherals.SPI1, sclk, mosi, peripherals.DMA_CH0, spi_config);
    let mut spi_device = ExclusiveDevice::new(spi, cs, del).unwrap();
        
    let mut display = sh1107::SH1107::new(&mut spi_device, dc, rst);

    // Set up thermometer
    //    
    // 



    let _ = display.init(&mut delay).await;
    delay.delay_ms(1000).await;

    let _ = display.clear().await;
    display.show().await;

    let _ = display.draw_rectangle(Point::new(0, 0), Size::new(128, 64), BinaryColor::On, false).await;
    //display.show().await;
    //delay.delay_ms(4000).await;

    let _ = display.draw_text("   AutoBrew rs ", Point::new(0, 22), BinaryColor::On).await;
    let _ = display.draw_text("     v0.1.0    ", Point::new(0, 40), BinaryColor::On).await;
    delay.delay_ms(10).await;
    display.show().await;
    delay.delay_ms(5000).await;
    display.clear().await;
    display.show().await;
    delay.delay_ms(10).await;

//    info!("Begin loop logic");
//    loop {




//    }
    
    info!("Program end");
}


