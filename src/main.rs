#![no_std]
#![no_main]

use defmt::*;
//use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
//use embedded_hal::digital::OutputPin;
use embassy_rp::gpio::{Level, Output};
//use embassy_rp::peripherals::DMA_CH0;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};
use embassy_time::{Duration, Timer, Delay};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::{SpiBus, SpiDevice};
//use oled_async::{prelude::*, Builder};
use display_interface::{DisplayError, AsyncWriteOnlyDataCommand };
use display_interface_spi::SPIInterface;
//use embedded_graphics::prelude::*;
//use embedded_graphics::primitives::{Line, PrimitiveStyle};
//use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyle};
//use embedded_graphics::text::Text;
//use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

//use oled_async::{prelude::*, Builder};

use auto_brew_rs::{adjustment, controls, sensor, sh1107};



//mod display_driver;
//use crate::display_driver::{displays, mode, builder};


#[cortex_m_rt::pre_init]
unsafe fn before_main() {
    // Soft-reset doesn't clear spinlocks. Clear the one used by critical-section
    // before we hit main to avoid deadlocks when using a debugger
    embassy_rp::pac::SIO.spinlock(31).write_value(1);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");
    let peripherals = embassy_rp::init(Default::default());
    let mut delay = Delay;

      
    let dc = Output::new(peripherals.PIN_8, Level::Low);     // Data/Command
    let cs = Output::new(peripherals.PIN_9, Level::Low);     // Chip Select
    let sclk = peripherals.PIN_10;                                      // Serial Clock
    let mosi = peripherals.PIN_11;                                      // Master Out Slave In
    let rst = Output::new(peripherals.PIN_12, Level::Low);  // Reset
    

    let mut spi_config = Config::default();
    spi_config.frequency = 2_000_000;  //sh1107::frequency();
    spi_config.phase = Phase::CaptureOnSecondTransition;
    spi_config.polarity = Polarity::IdleHigh;


    let spi = Spi::new_txonly(peripherals.SPI1, sclk, mosi, peripherals.DMA_CH0, spi_config);

        
    let mut display = sh1107::SH1107::new(spi, dc, rst, cs);
    
    let _ = display.init(&mut delay).await;
    delay.delay_ms(2000).await;

    let _ = display.clear().await;

    let _ = display.draw_rectangle(&mut delay, Point::new(1, 1), Size::new(126, 62), BinaryColor::On).await;
    delay.delay_ms(4000).await;

    //let _ = display.clear().await;
    
    info!("Program end");
}