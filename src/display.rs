
//use embedded_hal::spi::SpiDevice;
//use embassy_embedded_hal::shared_bus::asynch::spi::{SpiDevice, SpiDeviceWithConfig};
//use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
//use embedded_hal_async::delay::DelayNs;
//use embedded_hal_async::spi::{SpiBus, SpiDevice};
//use embedded_hal::spi::{Operation, SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
//use embassy_rp::PeripheralRef;
use embassy_rp::peripherals::{DMA_CH0, PIN_10, PIN_11, SPI1};
use embassy_rp::spi::{Async, Config, Spi};
use embassy_rp::gpio::Output;
use embassy_time::Delay;
use static_cell::StaticCell;
use crate::sh1107::SH1107;


pub struct DisplayPeripherals<'a, CLK, MOSI, SPI, DMA> {
    pub dc: Output<'a>,
    pub cs: Output<'a>,
    pub rst: Output<'a>,
    pub sclk: CLK,
    pub mosi: MOSI,
    pub inner: SPI,
    pub tx_dma: DMA,
}

impl<'a, CLK, MOSI, SPI, DMA> DisplayPeripherals<'a, CLK, MOSI, SPI, DMA> {
    pub fn new(
        dc: Output<'a>,
        cs: Output<'a>,
        rst: Output<'a>,
        sclk: CLK,
        mosi: MOSI,
        inner: SPI,
        tx_dma: DMA,
    ) -> Self {
        Self {
            dc,
            cs,
            rst,
            sclk,
            mosi,
            inner,
            tx_dma,
        }
    }
}

pub struct Display<'a> {
    display: SH1107<
        ExclusiveDevice<Spi<'a, SPI1, Async>, Output<'a>, Delay>,
        Output<'a>,
        Output<'a>
    >,
    delay: Delay,
}

//static SPI_DEVICE: StaticCell<ExclusiveDevice<Spi<SPI1, Async>, embassy_rp::gpio::Output<>, embassy_time::Delay>> = StaticCell::new();

impl<'a> Display<'a> {
    pub fn new<CLK, MOSI, SPI, DMA>(
        display_peripherals: DisplayPeripherals<'a, CLK, MOSI, SPI, DMA>
    ) -> Self
    where
        CLK: embassy_rp::Peripheral + 'a,
        CLK::P: embassy_rp::spi::ClkPin<SPI1>,
        MOSI: embassy_rp::Peripheral + 'a,
        MOSI::P: embassy_rp::spi::MosiPin<SPI1>,
        SPI: embassy_rp::Peripheral<P = SPI1> + 'a,
        DMA: embassy_rp::Peripheral<P = DMA_CH0> + 'a,
    {
        let delay = Delay;
        let DisplayPeripherals {
            dc,
            cs,
            rst,
            sclk,
            mosi,
            inner,
            tx_dma,
        } = display_peripherals;


        // SPI configuration
        let mut spi_config = Config::default();
        spi_config.frequency = 2_000_000;
        spi_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
        spi_config.polarity = embassy_rp::spi::Polarity::IdleHigh;

        let spi_device = ExclusiveDevice::new(
                Spi::new_txonly(
                    inner,
                    sclk,
                    mosi,
                    tx_dma,
                    spi_config
                ),
                cs,
                delay.clone()
            ).unwrap();

        // Initialize the display 
        let display = SH1107::new(
            spi_device,
            dc,
            rst,
        );
        /*
        let display = SH1107::new(
            spi_device,
            display_peripherals.dc,
            display_peripherals.rst
        );
        */
        
        Self {
            //display_peripherals,
            display,
            delay,
        }
    }

    pub async fn initialise(&mut self) {
        self.display.init(&mut self.delay).await.unwrap();
    }

    //pub async fn update_display(&mut self) {
    //    // Logic to update the display
    //    //println!("Updating display...");
    //}
}