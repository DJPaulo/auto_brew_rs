//use embedded_graphics::framebuffer::Framebuffer;
use embedded_hal_async::spi::SpiBus;
use embedded_hal_async::delay::DelayNs;
use embedded_hal::digital::{ErrorType, OutputPin};
use display_interface::DisplayError; //, AsyncWriteOnlyDataCommand};
//use display_interface_spi::SPIInterface;
//use display_interface_i2c::I2CInterface;
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::{Line, Rectangle, PrimitiveStyle, PrimitiveStyleBuilder};
use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyle};
//use embedded_graphics::text::Text;
//use embedded_graphics_framebuf::{FrameBuf, PixelIterator};



#[derive(Debug, Clone, Copy)]
pub struct SH1107<SPI, DC, RESET, CS> {
    spi: SPI,
    dc: DC,
    reset: RESET,
    cs: CS,
}

impl<SPI, DC, RESET, CS> SH1107<SPI, DC, RESET, CS>
where
    SPI: SpiBus,
    DC: OutputPin<Error = core::convert::Infallible>,
    RESET: OutputPin<Error = core::convert::Infallible>,
    CS: OutputPin<Error = core::convert::Infallible>,
{
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;


    pub fn new(spi: SPI, dc: DC, reset: RESET, cs: CS) -> Self {
        Self { spi, dc, reset, cs }
    }

    pub async fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DisplayError> {
        self.reset.set_high().map_err(|_| DisplayError::BusWriteError)?;
        delay.delay_ms(1).await;
        self.reset.set_low().map_err(|_| DisplayError::BusWriteError)?;
        delay.delay_ms(10).await;
        self.reset.set_high().map_err(|_| DisplayError::BusWriteError)?;

        let commands: &[u8] = &[
            0xAE, // Display OFF
            0x00, // Set lower column address
            0x10, // Set higher column address
            0xB0, // Set page address
            //0xDC, // Set display start line
            0x40, // Set display start line
            0x81, // Set contrast control
            0x6F, // 128
            0x21, // Set memory addressing mode (0x20/0x21)
            0xA0, // Set segment remap
            0xC0, // Set COM output scan direction
            0xA4, // Disable entire display on (0xA4 = false / 0xA5 = true)
            0xA6, // Set normal or inverse display
            0xA8, // Set multiplex ratio
            0x3F, // Duty = 1/64
            0xD3, 0x60, // Set display offset
            0xD5, 0x41, // Set display clock divide ratio / oscillator frequency
            0xD9, 0x22, // Set pre-charge period
            0xDB, 0x35, // Set VCOMH deselect level
            0xAD, // Set charge pump enable
            0x8A, // Set DC-DC control mode set enable
            0xAF, // Display ON
        ];

        self.send_commands(commands).await?;
        delay.delay_ms(10).await;
        Ok(())
    }

    pub async fn close<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DisplayError> {
        let commands: &[u8] = &[0xAE]; // Display OFF
        self.send_commands(commands).await?;
        Ok(())
    }
    
    async fn send_commands(&mut self, commands: &[u8]) -> Result<(), DisplayError> {
        self.cs.set_high().map_err(|_| DisplayError::BusWriteError)?;
        self.dc.set_low().map_err(|_| DisplayError::BusWriteError)?;
        self.cs.set_low().map_err(|_| DisplayError::BusWriteError)?;
        self.spi.write(commands).await.map_err(|_| DisplayError::BusWriteError)?;
        self.cs.set_high().map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        self.cs.set_high().map_err(|_| DisplayError::BusWriteError)?;
        self.dc.set_high().map_err(|_| DisplayError::BusWriteError)?;
        self.cs.set_low().map_err(|_| DisplayError::BusWriteError)?;
        self.spi.write(data).await.map_err(|_| DisplayError::BusWriteError)?;
        self.cs.set_high().map_err(|_| DisplayError::BusWriteError)?;
        Ok(())
    }

    async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        self.send_data(&[0xB0]).await?; // Set page address
        for page in 0..(Self::HEIGHT / 8) {     // 8 pixels per page
            let column = (Self::HEIGHT - 1) - page;
            self.send_commands(&[0x00 + (column & 0x0f)]).await?;
            self.send_commands(&[0x10 + (column >> 4)]).await?;
            for num in 0..(Self::HEIGHT / 4) {
                let index = (page * (Self::HEIGHT / 4)) + num;
                self.send_data(&[data[index as usize]]).await?;
            }
        }
        Ok(())
    }
    

    pub async fn clear(&mut self) -> Result<(), DisplayError> {
        let data = [0x00; ((Self::WIDTH as usize * Self::HEIGHT as usize) / 8) as usize];
        self.send_data(&data).await?;
        self.show(&data).await;
        Ok(())
    }

    pub async fn draw_rectangle<D: DelayNs>(&mut self, delay: &mut D, top_left: Point, bottom_right: Size, colour: BinaryColor) -> Result<(), DisplayError> {
        let style = PrimitiveStyleBuilder::new().stroke_color(colour).stroke_width(1).build();
        let rectangle = Rectangle::new(top_left, bottom_right).into_styled(style);
        let mut data = [0x00; ((Self::WIDTH as usize * Self::HEIGHT as usize) / 8) as usize];
        for Pixel(point, colour) in rectangle.pixels() { 
            let Point { x, y } = point;
            if x >= 0 && x < Self::WIDTH as i32 && y >= 0 && y < Self::HEIGHT as i32 { 
                let index = (y as usize * Self::WIDTH as usize + x as usize) / 8;
                if colour == BinaryColor::On {
                    data[index] |= 1 << (x % 8);
                } else { 
                    data[index] &= !(1 << (x % 8));
                }
            }
        }
        self.show(&data).await?;
        Ok(())
    }


}

/*
impl<SPI, DC, RESET> DrawTarget for Sh1107<SPI, DC, RESET>
where
    SPI: embedded_hal::blocking::spi::Write<u8>,
    DC: embedded_hal::digital::v2::OutputPin,
    RESET: embedded_hal::digital::v2::OutputPin,
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        // Draw pixels implementation
        Ok(())
    }
}
*/