use defmt::*;
//use embedded_graphics::framebuffer::Framebuffer;
use display_interface::{DataFormat, DisplayError, AsyncWriteOnlyDataCommand};
use embedded_hal::digital::{ErrorType, OutputPin};
//use embedded_hal::spi::SpiDevice;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::{SpiBus, SpiDevice}; 
                                     //use display_interface_spi::SPIInterface;
                                     //use display_interface_i2c::I2CInterface;
use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
//use embedded_graphics::text::Text;
// use embedded_graphics_framebuf::{FrameBuf, PixelIterator};


/* #[derive(Clone, Copy)]
pub enum NoOutputPin {}

impl OutputPin for NoOutputPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ErrorType for NoOutputPin {
    type Error = core::convert::Infallible;
} */

pub trait DisplaySize {
    const WIDTH: u8;
    const HEIGHT: u8;

    fn dimensions() -> (u8, u8) {
        (Self::WIDTH, Self::HEIGHT)
    }

    /// Initialise the display for column mode
    #[allow(async_fn_in_trait)]
    async fn init<DI>(intf: &mut DI) -> Result<(), DisplayError>
    where
        DI: AsyncWriteOnlyDataCommand;
}




#[derive(Debug, Clone, Copy)]
pub struct SH1107_128_64 {}

impl DisplaySize for SH1107_128_64 {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;
    // const BUFFER_SIZE: usize = (Self::WIDTH as usize * Self::HEIGHT as usize) / 8;

    async fn init<DI>(intf: &mut DI) -> Result<(), DisplayError>
        where
            DI: AsyncWriteOnlyDataCommand, {
                let mut delay = DelayNs;
                //let _ = reset(delay).await;
                let _ = turn_off(&mut intf).await;
                intf.send_commands(DataFormat::U8(&[0x00])).await?; // Set lower column address
                /* self.send_commands(&[0x10]).await?; // Set higher column address
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
                self.send_commands(&[0xD5, 0x50]).await?; // Divide ratio/oscillator frequency mode
                self.send_commands(&[0xD9, 0x22]).await?; // Set discharge / precharge period
                self.send_commands(&[0xDB, 0x35]).await?; // Set VCOM deselect level
                self.send_commands(&[0xAD, 0x81]).await?; // Set DC-DC control mode (0x81 = On / 0x80 = Off)
                self.turn_on(delay).await; */
                Ok(())
    }

}

/*
async fn reset<DI, D>(intf: &mut DI, delay: &mut D) -> Result<(), DisplayError> {
    self.rst
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    delay.delay_ms(1).await;
    self.rst
        .set_low()
        .map_err(|_| DisplayError::BusWriteError)?;
    delay.delay_ms(10).await;
    self.rst
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    delay.delay_ms(10).await;
    Ok(())
}
*/

pub async fn turn_on<DI, D>(intf: &mut DI, delay: &mut D) -> Result<(), DisplayError> {
    intf.send_commands(&[0xAF]).await?; // Display ON
    delay.delay_ms(10).await;
    Ok(())
}

pub async fn turn_off<DI>(intf: &mut DI) -> Result<(), DisplayError> {
    intf.send_commands(&[0xAE]).await?; // Display OFF
    Ok(())
}
/*
async fn send_commands(&mut self, commands: &[u8]) -> Result<(), DisplayError> {
    self.cs
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.dc
        .set_low()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.cs
        .set_low()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.spi
        .write(commands)
        .await
        .map_err(|_| DisplayError::BusWriteError)?;
    self.cs
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    Ok(())
}

async fn send_data(&mut self, data: &[u8]) -> Result<(), DisplayError> {
    self.cs
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.dc
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.cs
        .set_low()
        .map_err(|_| DisplayError::BusWriteError)?;
    self.spi
        .write(data)
        .await
        .map_err(|_| DisplayError::BusWriteError)?;
    self.cs
        .set_high()
        .map_err(|_| DisplayError::BusWriteError)?;
    Ok(())
}


async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {
    let row_bytes = (Self::WIDTH / 8) as usize;
    let pages = (Self::HEIGHT / 8) as usize;
    let pages_to_update = (1 << pages) - 1;
    let mut current_page = 1;

    for start_row in (0..pages * 8).step_by(8) {
        if pages_to_update & current_page != 0 {
            for row in start_row..start_row + 8 {
                self.send_commands(&[(row as u8 & 0x0F), (0x10 | (row >> 4) as u8)])
                    .await?;
                let slice_start: usize = row * row_bytes;
                self.send_data(&data[slice_start..slice_start + row_bytes])
                    .await?;
            }
        }
        current_page <<= 1;
    }
    Ok(())
}
*/

async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {
    let row_bytes = (Self::WIDTH / 8) as usize;
    let pages = (Self::HEIGHT / 8) as usize;

    for page in 0..pages {
        for row in 0..8 {
            let start_row = page * 8 + row;
            self.send_commands(&[(start_row as u8 & 0x0F), (0x10 | (start_row >> 4) as u8)])
                .await?;
            let slice_start: usize = start_row * row_bytes;
            self.send_data(&data[slice_start..slice_start + row_bytes])
                .await?;
        }
    }
    Ok(())
}



pub async fn clear(&mut self) -> Result<(), DisplayError> {
    let data = [0x00; Self::BUFFER_SIZE];
    self.show(&data).await;
    Ok(())
}

pub async fn draw_rectangle(
    &mut self,
    top_left: Point,
    size: Size,
    colour: BinaryColor,
) -> Result<(), DisplayError> {
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(colour)
        .stroke_width(1)
        .build();

    let rectangle = Rectangle::new(top_left, size).into_styled(style);
        
    let mut data = [0x00; Self::BUFFER_SIZE];
    //self.buffer = [0x00; Self::BUFFER_SIZE];
    for Pixel(point, colour) in rectangle.pixels() {
        let Point { x, y } = point;
        if x >= 0 && x < Self::WIDTH as i32 && y >= 0 && y < Self::HEIGHT as i32 {
            let index = ((y as usize / 8) * Self::WIDTH as usize) + x as usize;
            let bit = 1 << (y % 8);
            if colour == BinaryColor::On {
                data[index] |= bit;
            } else {
                data[index] &= !bit;
            }

            //let index = (y as usize * Self::WIDTH as usize + x as usize) / 8;
            //if colour == BinaryColor::On {
            //    data[index] |= 1 << (x % 8);
            //} else {
            //    data[index] &= !(1 << (x % 8));
            //}
        }
        self.show(&data).await?;
        //info!("Drawing at x: {}, y: {}, index: {}", x, y, index);
    }
    //self.show(&data).await?;
    

    Ok(())
}

pub async fn draw_line(
    &mut self,
    start: Point,
    end: Point,
    colour: BinaryColor,
) -> Result<(), DisplayError> {
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(colour)
        .stroke_width(1)
        .build();

    let line = Line::new(start, end).into_styled(style);

    let mut data = [0x00; Self::BUFFER_SIZE];
    for Pixel(coord, pixel_colour) in line.pixels() {
        let x = coord.x;
        let y = coord.y;
        if x >= 0 && x < Self::WIDTH as i32 && y >= 0 && y < Self::HEIGHT as i32 {
            let index = ((y / 8) as usize * Self::WIDTH as usize) + x as usize; // Calculate byte index in buffer
            let bit = 1 << (y % 8); // Determine the bit position within the byte
            if pixel_colour == BinaryColor::On {
                data[index] |= bit;
            } else {
                data[index] &= !bit;
            }
        }
        self.show(&data).await?;
    }

    Ok(())
}





/*
    async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        self.send_commands(&[0xB0 | 0, 0x00, 0x10]).await?; // Set page address
        //self.send_commands(&[0x00 & 0x0F]).await?;
        //self.send_commands(&[0x10 >> 4]).await?;
        let start_index = 0 as usize;
        let end_index = start_index + 128 as usize;
        let mut buf = [0xFF; Self::BUFFER_SIZE];
        self.send_data(&data[start_index..end_index]).await?;
        Ok(())
    }


    async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        self.send_commands(&[0xB0]).await?; // Set page address
        for row in 0..(Self::HEIGHT) {
            let column = (Self::HEIGHT - 1) - row;
            self.send_commands(&[0x00 + (column & 0x0F)]).await?;
            self.send_commands(&[0x10 + (column >> 4)]).await?;
            for num in 0..(Self::WIDTH) {
                let start_index = row as usize * Self::WIDTH as usize;
                let end_index = start_index as usize + Self::WIDTH as usize;
                self.send_data(&data[start_index..end_index]).await?;
            }
        }
        Ok(())
    }
*/

/*
    async fn show(&mut self, data: &[u8]) -> Result<(), DisplayError> {

        for page in 0..(Self::HEIGHT / 8) { // 8 pixels per page
            self.send_commands(&[0xB0 | page]).await?; // Set page address

            //self.send_commands(&[
            //    0xB0 | page,      // Set page address
            //    0x00,             // Set lower column address
            //    0x10              // Set higher column address
            //]).await?;

            let start_index = page as usize * Self::WIDTH as usize;
            let end_index = start_index as usize + Self::WIDTH as usize;

            for column in 0..(Self::WIDTH - 120) {
                let lower_column_address = 0x00 | (column & 0x07);
                let higher_column_address = 0x10 | ((column >> 4));
                self.send_commands(&[lower_column_address, higher_column_address]).await?;
                //self.send_data(&[data[start_index + column as usize]]).await?;
                self.send_data(&data[start_index..end_index]).await?;
                info!("Page: {}, Column: {}", page, column);
                info!("Showing at low: {}, high: {}, start index: {}, end index: {}", lower_column_address, higher_column_address, start_index, end_index);
            }

        }
        Ok(())
    }
*/

/*
/// SPI communication error
#[derive(Debug)]
struct CommError;

/// A fake 64px x 64px display.
struct ExampleDisplay {
    /// The framebuffer with one `u8` value per pixel.
    framebuffer: [u8; 64 * 64],

    /// The interface to the display controller.
    iface: SPI1,
}

impl ExampleDisplay {
    /// Updates the display from the framebuffer.
    pub fn flush(&self) -> Result<(), CommError> {
        self.iface.send_bytes(&self.framebuffer)
    }
}

impl DrawTarget for ExampleDisplay {
    type Color = Gray8;
    // `ExampleDisplay` uses a framebuffer and doesn't need to communicate with the display
    // controller to draw pixel, which means that drawing operations can never fail. To reflect
    // this the type `Infallible` was chosen as the `Error` type.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (63,63)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x @ 0..=63, y @ 0..=63)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let index: u32 = x + y * 64;
                self.framebuffer[index as usize] = color.luma();
            }
        }

        Ok(())
    }
}

impl OriginDimensions for ExampleDisplay {
    fn size(&self) -> Size {
        Size::new(64, 64)
    }
}

let mut display = ExampleDisplay {
    framebuffer: [0; 4096],
    iface: SPI1,
};

// Draw a circle with top-left at `(22, 22)` with a diameter of `20` and a white stroke
let circle = Circle::new(Point::new(22, 22), 20)
    .into_styled(PrimitiveStyle::with_stroke(Gray8::WHITE, 1));

circle.draw(&mut display)?;

// Update the display
display.flush().unwrap();
*/