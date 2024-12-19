use embedded_hal::spi::SpiDevice;
use embedded_hal_async::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use display_interface::{DisplayError, AsyncWriteOnlyDataCommand, DataFormat};
//use display_interface_spi::SPIInterface;
//use display_interface_i2c::I2CInterface;
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::{Rectangle, PrimitiveStyleBuilder};
use embedded_graphics::mono_font::{ascii::FONT_8X13, MonoTextStyle};
use embedded_graphics::text::Text;


#[derive(Debug, Clone, Copy)]
pub struct SH1107<SPI, DC, RESET> {
    spi: SPI,
    dc: DC,
    rst: RESET,
    buffer: [u8; BUFFER_SIZE],
}

const WIDTH: u8 = 128;
const HEIGHT: u8 = 64;
const BUFFER_SIZE: usize = (WIDTH as usize * HEIGHT as usize) / 8;

impl<SPI, DC, RESET> SH1107<SPI, DC, RESET>
where
    SPI: SpiDevice,
    DC: OutputPin<Error = core::convert::Infallible>,
    RESET: OutputPin<Error = core::convert::Infallible>,
{

    pub fn new(spi: SPI, dc: DC, rst: RESET) -> Self {
        Self {
            spi,
            dc,
            rst,
            buffer: [0; BUFFER_SIZE],
         }
    }

    pub async fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DisplayError> {
        self.reset(delay).await?;
        self.off().await;
        self.send_commands(&[0x00]).await?; // Set lower column address
        self.send_commands(&[0x10]).await?; // Set higher column address
        self.send_commands(&[0xB0]).await?; // Set page address
        self.send_commands(&[0xDC, 0x00]).await?; // Set display start line
        self.send_commands(&[0x81, 0x6F]).await?; // Set contrast control
        self.send_commands(&[0x21]).await?; // Set memory addressing mode (0x20 = Horizontal / 0x21 = Vertical)
        self.send_commands(&[0xA0]).await?; // Set segment remap (0xA0 / 0xA1)
        self.send_commands(&[0xC0]).await?; // Set common output scan direction (0xC0 / 0xC8)
        self.send_commands(&[0xA4]).await?; // Set entire display on (0xA4 = false / 0xA5 = true)
        self.send_commands(&[0xA6]).await?; // Set normal or reverse display on (0xA6 = Normal / 0xA7 = Reverse)
        self.send_commands(&[0xA8, 0x3F]).await?; // Set multiplex ratio (Display height - 1)
        self.send_commands(&[0xD3, 0x60]).await?; // Set display offset
        self.send_commands(&[0xD5, 0x50]).await?; // Clock divide ratio / oscillator frequency mode
        self.send_commands(&[0xD9, 0x22]).await?; // Set discharge / precharge period
        self.send_commands(&[0xDB, 0x35]).await?; // Set VCOM deselect level
        self.send_commands(&[0xAD, 0x81]).await?; // Set DC-DC control mode (0x81 = On / 0x80 = Off)
        self.on().await;
        delay.delay_ms(10).await;
        Ok(())
    }

    async fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DisplayError> {
        self.rst.set_high().map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(1).await;
        self.rst.set_low().map_err(|_| DisplayError::RSError)?;
        delay.delay_ms(10).await;
        self.rst.set_high().map_err(|_| DisplayError::RSError)?;
        Ok(())
    }

    pub async fn off(&mut self) -> Result<(), DisplayError> {
        let commands: &[u8] = &[0xAE]; // Display OFF
        self.send_commands(commands).await?;
        Ok(())
    }

    pub async fn on(&mut self) -> Result<(), DisplayError> {
        let commands: &[u8] = &[0xAF]; // Display ON
        self.send_commands(commands).await?;
        Ok(())
    }
    
    async fn send_commands(&mut self, commands: &[u8]) -> Result<(), DisplayError> {
        //self.cs.set_high().map_err(|_| DisplayError::CSError)?;
        self.dc.set_low().map_err(|_| DisplayError::DCError)?;
        //self.cs.set_low().map_err(|_| DisplayError::CSError)?;
        self.spi.write(commands).map_err(|_| DisplayError::BusWriteError)?;
        //self.cs.set_high().map_err(|_| DisplayError::CSError)?;
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        //self.cs.set_high().map_err(|_| DisplayError::CSError)?;
        self.dc.set_high().map_err(|_| DisplayError::DCError)?;
        //self.cs.set_low().map_err(|_| DisplayError::CSError)?;
        self.spi.write(data).map_err(|_| DisplayError::BusWriteError)?;
        //self.cs.set_high().map_err(|_| DisplayError::CSError)?;
        Ok(())
    }

    pub async fn show(&mut self) -> Result<(), DisplayError> {
        self.send_commands(&[0xB0]).await?; // Set page address
        for page in 0..64 {     // 64 rows
            let column = (HEIGHT - 1) - page;
            self.send_commands(&[0x00 + (column & 0x0f)]).await?;
            self.send_commands(&[0x10 + (column >> 4)]).await?;
            for num in 0..16 { // 16 pages of 8 bit each
                let index = (page as usize * 16 as usize) + num;
                self.send_data(&[self.buffer[index as usize]]).await?;
            }
        }
        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), DisplayError> {
        self.buffer.fill(0x00);
        Ok(())
    }

    pub async fn draw_rectangle(&mut self, top_left: Point, bottom_right: Size, colour: BinaryColor, fill: bool) -> Result<(), DisplayError> {
        let style = if fill {
            PrimitiveStyleBuilder::new().stroke_color(colour).stroke_width(1).fill_color(colour).build()
        } else {
            PrimitiveStyleBuilder::new().stroke_color(colour).stroke_width(1).build()
        };
        let rectangle = Rectangle::new(top_left, bottom_right).into_styled(style);
        rectangle.draw(self)?;
        
        /*for Pixel(point, colour) in rectangle.pixels() { 
            let Point { x, y } = point;
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 { 
                let index = (y as usize * WIDTH as usize + x as usize) / 8;
                if colour == BinaryColor::On {
                    self.buffer[index] |= 1 << (x % 8);
                } else { 
                    self.buffer[index] &= !(1 << (x % 8));
                }
            }
        }*/
        Ok(())
    }

    pub async fn draw_text(&mut self, text: &str, top_left: Point, colour: BinaryColor) -> Result<(), DisplayError> {
        let style = MonoTextStyle::new(&FONT_8X13, colour);
        let txt = Text::new(&text, top_left, style);
        txt.draw(self)?;
        Ok(())
    }


}


impl<SPI, DC, RESET> DrawTarget for SH1107<SPI, DC, RESET>
where
    SPI: SpiDevice,
    DC: OutputPin<Error = core::convert::Infallible>,
    RESET: OutputPin<Error = core::convert::Infallible>,
{
    type Color = BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, colour) in pixels {
            let (x, y) = (coord.x, coord.y);
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 { 
                let index = (y as usize * WIDTH as usize + x as usize) / 8;
                if colour == BinaryColor::On {
                    self.buffer[index] |= 1 << (x % 8);
                } else { 
                    self.buffer[index] &= !(1 << (x % 8));
                }
            }
        }
        // Send the accumulated data to the display
        self.send_commands(&[0xB0]); // Set page address
        for page in 0..64 { // 64 rows
            let column = (HEIGHT - 1) - page;
            self.send_commands(&[0x00 + (column & 0x0f)]);
            self.send_commands(&[0x10 + (column >> 4)]);
            for num in 0..16 { // 16 pages of 8 bit each
                let index = (page as usize * 16 as usize) + num;
                self.send_data(&[self.buffer[index as usize]]);
            }
        }
        Ok(())
    }
}

impl<SPI, DC, RST> Dimensions for SH1107<SPI, DC, RST> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), Size::new(128, 64))
    }
}
