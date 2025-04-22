//use defmt::Str;
use embedded_hal_bus::spi::ExclusiveDevice;
use embassy_rp::peripherals::{DMA_CH0, SPI1};
use embassy_rp::spi::{Async, Config, Spi};
use embassy_rp::gpio::Output;
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::BinaryColor;

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
        
        Self {
            display,
            delay,
        }
    }

    pub async fn initialise(&mut self) {
        self.display.init(&mut self.delay).await.unwrap();
    }

    pub async fn clear_all(&mut self) {
        let _ = self.display.clear().await;
    }

    pub async fn clear_line_1(&mut self) {
        let _ = self.display.draw_rectangle(Point::new(0, 0), Size::new(128, 16), BinaryColor::Off, true).await;
    }

    pub async fn clear_line_2(&mut self) {
        let _ = self.display.draw_rectangle(Point::new(0, 16), Size::new(128, 16), BinaryColor::Off, true).await;
    }

    pub async fn clear_line_3(&mut self) {
        let _ = self.display.draw_rectangle(Point::new(0, 32), Size::new(128, 16), BinaryColor::Off, true).await;
    }

    pub async fn clear_line_4(&mut self) {
        let _ = self.display.draw_rectangle(Point::new(0, 48), Size::new(128, 16), BinaryColor::Off, true).await;
    }

    pub async fn refresh_line_1(&mut self, text: &str) {
        let _ = self.clear_line_1().await;
        let display_line = " Current: ";
        let _ = self.display.draw_text(display_line, Point::new(0, 10), BinaryColor::On).await;
        let _ = self.display.draw_text(text, Point::new(75, 10), BinaryColor::On).await;
    }

    pub async fn refresh_line_2(&mut self, text: &str) {
        let _ = self.clear_line_2().await;
        let display_line = "  Target: ";
        let _ = self.display.draw_text(display_line, Point::new(0, 26), BinaryColor::On).await;
        let _ = self.display.draw_text(text, Point::new(75, 26), BinaryColor::On).await;
    }

    pub async fn refresh_line_3(&mut self, text: &str) {
        let _ = self.clear_line_3().await;
        let display_line = "    Diff: ";
        let _ = self.display.draw_text(display_line, Point::new(0, 42), BinaryColor::On).await;
        let _ = self.display.draw_text(text, Point::new(75, 42), BinaryColor::On).await;
    }

    pub async fn refresh_line_4(&mut self, text: &str) {
        let _ = self.clear_line_4().await;
        let _ = self.display.draw_text(text, Point::new(0, 58), BinaryColor::On).await;
    }

    pub async fn show(&mut self) {
        let _ = self.display.show().await;
    }

    pub async fn show_splash_screen(&mut self) {
        let _ = self.display.clear().await;
        let _ = self.display.show().await;
        let _ = self.display.draw_rectangle(Point::new(0, 0), Size::new(128, 64), BinaryColor::On, false).await;
        let _ = self.display.draw_text("   AutoBrew rs ", Point::new(0, 22), BinaryColor::On).await;
        let _ = self.display.draw_text("     v0.1.0    ", Point::new(0, 40), BinaryColor::On).await;
        Timer::after(Duration::from_millis(10)).await;
        let _ = self.display.show().await;
        Timer::after(Duration::from_millis(5000)).await;
        let _ = self.display.clear().await;
        let _ = self.display.show().await;
        Timer::after(Duration::from_millis(10)).await;
    }

    pub async fn refresh_readings(&mut self, cur_tmp: &str, tar_tmp: &str, cur_var: &str, msg: &str) {
        if msg.len() > 0 {
            let _ = self.refresh_line_4(msg).await;
            let _ = self.show().await;
        }
        else {
            let _ = self.refresh_line_1(cur_tmp).await;
            let _ = self.refresh_line_2(tar_tmp).await;
            let _ = self.refresh_line_3(cur_var).await;
            let _ = self.clear_line_4().await;
            let _ = self.display.show().await;
        }
    }
}