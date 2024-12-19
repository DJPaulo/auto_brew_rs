//use crate::{DISPLAY_TIMEOUT};

//use display_interface::{AsyncWriteOnlyDataCommand, DataFormat, DisplayError};

//use crate::display_driver::builder;
//use crate::display_driver::sh1107;
use embassy_rp::spi::{Config, Phase, Polarity, Spi};
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Timer, Delay};
use embedded_hal::digital::OutputPin;
//use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use display_interface::DisplayError;

//use embassy_rp::Peripheral;
use crate::sh1107;

#[derive(Debug, Clone, Copy)]
pub struct Display {
    //driver: sh1107::SH1107,
}

impl Display {

    pub async fn new() -> Self {
        Self {
      //      driver: sh1107::SH1107,
         }
    }


    pub async fn init() -> Result<(), DisplayError> {
        let peripherals = embassy_rp::init(Default::default());
        let mut delay = Delay;
        //let mut del = Delay;

        let dc = Output::new(peripherals.PIN_8, Level::Low);     // Data/Command
        let cs = Output::new(peripherals.PIN_9, Level::High);    // Chip Select
        let sclk = peripherals.PIN_10;                                      // Serial Clock
        let mosi = peripherals.PIN_11;                                      // Master Out Slave In
        let rst = Output::new(peripherals.PIN_12, Level::Low);  // Reset

        let mut spi_config = Config::default();
        spi_config.frequency = 2_000_000;
        spi_config.phase = Phase::CaptureOnSecondTransition;
        spi_config.polarity = Polarity::IdleHigh;

        let spi = Spi::new_txonly(peripherals.SPI1, sclk, mosi, peripherals.DMA_CH0, spi_config);
        let mut spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
            
        let mut display = sh1107::SH1107::new(&mut spi_device, dc, rst);
        Ok(())
        // display.init(&mut delay).await?;
        //Ok()

    }

}