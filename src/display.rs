
//use embedded_hal::spi::SpiDevice;
//use embassy_embedded_hal::shared_bus::asynch::spi::{SpiDevice, SpiDeviceWithConfig};
//use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
//use embedded_hal_async::delay::DelayNs;
//use embedded_hal_async::spi::{SpiBus, SpiDevice};
//use embedded_hal::spi::{Operation, SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use embassy_rp::PeripheralRef;
use embassy_rp::peripherals::{DMA_CH0, PIN_10, PIN_11, SPI1};
use embassy_rp::spi::{Async, Config, Spi};
use embassy_rp::gpio::Output;
use embassy_time::Delay;
use static_cell::StaticCell;
use crate::sh1107::SH1107;


pub struct DisplayPeripherals<'a> {
    pub dc: Output<'a>,
    pub cs: Output<'a>,
    pub sclk: &'a mut PIN_10,   // <-- Can this be more generic?
    pub mosi: &'a mut PIN_11,   // <-- Can this be more generic?
    pub rst: Output<'a>,
    pub inner: &'a mut SPI1,    // <-- Can this be more generic?
    pub tx_dma: &'a mut DMA_CH0 // <-- Can this be more generic?
}
pub struct Display<'a> {
    display_peripherals: &'a mut DisplayPeripherals<'a>,
    //spi_device: &mut ExclusiveDevice<Spi<'a, SPI1, Async>, Output<'a>, Delay>,
    display: SH1107<embedded_hal_bus::spi::ExclusiveDevice<Spi<'a, SPI1, Async>, &'a mut embassy_rp::gpio::Output<'a>, embassy_time::Delay>, &'a mut embassy_rp::gpio::Output<'a>, &'a mut embassy_rp::gpio::Output<'a>>,
    delay: Delay,
}

//static SPI_DEVICE: StaticCell<ExclusiveDevice<Spi<SPI1, Async>, embassy_rp::gpio::Output<>, embassy_time::Delay>> = StaticCell::new();

impl<'a> Display<'a> {
    pub fn new(display_peripherals: &'a mut DisplayPeripherals<'a>) -> Self {
        let delay = Delay;

        // Initialize display pins
        //let dc = embassy_rp::gpio::Output::new(&mut peripherals.PIN_8, embassy_rp::gpio::Level::Low); // Data/Command
        //let cs = embassy_rp::gpio::Output::new(&mut peripherals.PIN_9, embassy_rp::gpio::Level::High); // Chip Select
        //let sclk = &mut peripherals.PIN_10; // Serial Clock
        //let mosi = &mut peripherals.PIN_11; // Master Out Slave In
        //let rst = embassy_rp::gpio::Output::new(&mut peripherals.PIN_12, embassy_rp::gpio::Level::Low); // Reset
        //let inner = &mut peripherals.SPI1;
        //let tx_dma = &mut peripherals.DMA_CH0;


        // SPI configuration
        let mut spi_config = Config::default();
        spi_config.frequency = 2_000_000;
        spi_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
        spi_config.polarity = embassy_rp::spi::Polarity::IdleHigh;

        // Initialize SPI
        //let spi_bus = Spi::new_txonly(display_peripherals.inner, display_peripherals.sclk, display_peripherals.mosi, display_peripherals.tx_dma, spi_config);
        //let spi_device = SPI_DEVICE.init(ExclusiveDevice::new(spi, display_peripherals.cs, delay).unwrap()); 
        // TODO: replace unwrap with error handling
        //let mut spi_device = match ExclusiveDevice::new(spi, cs, delay) {
        //    Ok(device) => device,
        //    Err(e) => {
        //        return Display { display: None, delay: delay};
        //    }
        //};
        let spi_device = ExclusiveDevice::new(
            Spi::new_txonly(&mut display_peripherals.inner, &mut display_peripherals.sclk, &mut display_peripherals.mosi, &mut display_peripherals.tx_dma, spi_config.clone()),
            &mut display_peripherals.cs,
            delay.clone()).unwrap();
        // Initialize the display 
        let display = SH1107::new(
            spi_device,
            &mut display_peripherals.dc,
            &mut display_peripherals.rst
        );
        /*
        let display = SH1107::new(
            spi_device,
            display_peripherals.dc,
            display_peripherals.rst
        );
        */

        Self {display_peripherals, display, delay}
    }

    pub async fn initialize(&mut self) {
        self.display.init(&mut self.delay).await.unwrap();
    }

    //pub async fn update_display(&mut self) {
    //    // Logic to update the display
    //    //println!("Updating display...");
    //}
}