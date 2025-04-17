
use embassy_rp::pio;
use embassy_rp::pio_programs::onewire::PioOneWire; //, PioOneWireProgram}};

/// Resolution settings for temperature readings
#[derive(Copy, Clone)]
pub enum Resolution {
    Bits9  = 0x1F, // 0.5°C resolution, 93.75ms conversion time
    Bits10 = 0x3F, // 0.25°C resolution, 187.5ms conversion time
    Bits11 = 0x5F, // 0.125°C resolution, 375ms conversion time
    Bits12 = 0x7F, // 0.0625°C resolution, 750ms conversion time
}

/// DS18B20 temperature sensor driver
pub struct Ds18b20<'d, PIO: pio::Instance, const SM: usize> {
    wire: PioOneWire<'d, PIO, SM>,
}

impl<'d, PIO: pio::Instance, const SM: usize> Ds18b20<'d, PIO, SM> {
    pub fn new(wire: PioOneWire<'d, PIO, SM>) -> Self {
        Self { wire }
    }

    pub async fn search_for_roms(&mut self) -> Result<[Option<[u8; 8]>; 8], ()> {
        let mut devices = [None; 8];  // Max 8 devices supported
        let mut device_count = 0;
        let mut last_discrepancy = 0;
        let mut rom_bits = [false; 64];
    
        'search: loop {
            let mut current_rom = [0u8; 8];
            let mut current_bit = 0;
            let mut discrepancy_marker = 0;
    
            // Send Search ROM command
            self.wire.write_bytes(&[0xF0]).await;
    
            // Read all 64 bits
            for byte in 0..8 {
                for bit in 0..8 {
                    // Read two bits
                    let mut bit_buffer = [0u8; 1];
                    self.wire.read_bytes(&mut bit_buffer).await;
                    let id_bit = (bit_buffer[0] & 0x01) != 0;
                    self.wire.read_bytes(&mut bit_buffer).await;
                    let complement_bit = (bit_buffer[0] & 0x01) != 0;
    
                    // Process bit results
                    if !id_bit && !complement_bit {
                        if current_bit == last_discrepancy {
                            rom_bits[current_bit] = true;
                        } else if current_bit > last_discrepancy {
                            rom_bits[current_bit] = false;
                            discrepancy_marker = current_bit;
                        }
                        // Remove self-assignment, the value stays unchanged in the else case
                    } else if id_bit && !complement_bit {
                        rom_bits[current_bit] = true;
                    } else if !id_bit && complement_bit {
                        rom_bits[current_bit] = false;
                    } else {
                        // Invalid response
                        return Err(());
                    }
    
                    // Write the bit
                    let write_bit = if rom_bits[current_bit] { 1u8 } else { 0u8 };
                    self.wire.write_bytes(&[write_bit]).await;
    
                    // Store bit in current_rom
                    if rom_bits[current_bit] {
                        current_rom[byte] |= 1 << bit;
                    }
    
                    current_bit += 1;
                }
            }

            // Verify CRC
            if Self::crc8(&current_rom[0..7]) == current_rom[7] {
                devices[device_count] = Some(current_rom);
                device_count += 1;
            }

            last_discrepancy = discrepancy_marker;
            if last_discrepancy == 0 || device_count >= 8 {
                break 'search;
            }
        }

        Ok(devices)
    }

    /// Set the resolution for a specific device
    pub async fn set_resolution_with_rom(&mut self, rom: &[u8; 8], resolution: Resolution) -> Result<(), ()> {
        // Match ROM command followed by ROM code
        self.wire.write_bytes(&[0x55]).await;
        self.wire.write_bytes(rom).await;

        // Write to configuration register
        self.wire.write_bytes(&[0x4E]).await;               // Write Scratchpad
        self.wire.write_bytes(&[0x00]).await;               // Th register
        self.wire.write_bytes(&[0x00]).await;               // Tl register
        self.wire.write_bytes(&[resolution as u8]).await;   // Configuration register

        // Read back scratchpad to verify
        self.wire.write_bytes(&[0xBE]).await;
        let mut data = [0; 9];
        self.wire.read_bytes(&mut data).await;
        
        if Self::crc8(&data) == 0 && (data[4] & 0x60) == (resolution as u8 & 0x60) {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Set the resolution for all devices (broadcast)
    pub async fn set_resolution(&mut self, resolution: Resolution) -> Result<(), ()> {
        // Skip ROM command (broadcast to all devices)
        self.wire.write_bytes(&[0xCC]).await;

        // Write to configuration register
        self.wire.write_bytes(&[0x4E]).await;               // Write Scratchpad
        self.wire.write_bytes(&[0x00]).await;               // Th register
        self.wire.write_bytes(&[0x00]).await;               // Tl register
        self.wire.write_bytes(&[resolution as u8]).await;   // Configuration register

        Ok(())
    }

    /// Read the unique 64-bit ROM code of the sensor.
    /// This should only be used when there is a single device on the bus.
    /// Returns an array of 8 bytes: [family_code, serial(6), crc]
    // async fn read_rom(&mut self) -> Result<[u8; 8], ()> {
    //     // Send Read ROM command
    //     self.wire.write_bytes(&[0x33]).await;
        
    //     // Read 8 bytes (64-bit ROM code)
    //     let mut rom_code = [0u8; 8];
    //     self.wire.read_bytes(&mut rom_code).await;
        
    //     // Verify CRC
    //     if Self::crc8(&rom_code[0..7]) == rom_code[7] {
    //         Ok(rom_code)
    //     } else {
    //         Err(())
    //     }
    // }

    /// Format the ROM code as a hex string
    pub fn format_rom(rom: &[u8; 8]) -> [u8; 16] {
        let mut hex = [0u8; 16];
        for i in 0..8 {
            let byte = rom[i];
            hex[i*2] = match byte >> 4 {
                0..=9 => b'0' + (byte >> 4),
                _ => b'A' + (byte >> 4) - 10,
            };
            hex[i*2+1] = match byte & 0xF {
                0..=9 => b'0' + (byte & 0xF),
                _ => b'A' + (byte & 0xF) - 10,
            };
        }
        hex
    }

    /// Calculate CRC8 of the data
    fn crc8(data: &[u8]) -> u8 {
        let mut temp;
        let mut data_byte;
        let mut crc = 0;
        for b in data {
            data_byte = *b;
            for _ in 0..8 {
                temp = (crc ^ data_byte) & 0x01;
                crc >>= 1;
                if temp != 0 {
                    crc ^= 0x8C;
                }
                data_byte >>= 1;
            }
        }
        crc
    }

    /// Start a new measurement for a specific device. Allow at least 1000ms before getting `temperature`.
    pub async fn start_with_rom(&mut self, rom: &[u8; 8]) {
        // Match ROM command followed by ROM code
        self.wire.write_bytes(&[0x55]).await;
        self.wire.write_bytes(rom).await;
        // Start conversion
        self.wire.write_bytes(&[0x44]).await;
    }

    /// Start a new measurement for all devices. Allow at least 1000ms before getting `temperature`.
    pub async fn start(&mut self) {
        self.wire.write_bytes(&[0xCC, 0x44]).await;
    }

    /// Read the temperature from a specific device. Ensure >1000ms has passed since `start` before calling this.
    pub async fn temperature_with_rom(&mut self, rom: &[u8; 8]) -> Result<f32, ()> {
        // Match ROM command followed by ROM code
        self.wire.write_bytes(&[0x55]).await;
        self.wire.write_bytes(rom).await;
        // Read scratchpad
        self.wire.write_bytes(&[0xBE]).await;
        let mut data = [0; 9];
        self.wire.read_bytes(&mut data).await;
        match Self::crc8(&data) == 0 {
            true => Ok(((data[1] as u32) << 8 | data[0] as u32) as f32 / 16.),
            false => Err(()),
        }
    }

    /// Read the temperature. (Only works if there is one device) Ensure >1000ms has passed since `start` before calling this.
    pub async fn temperature(&mut self) -> Result<f32, ()> {
        self.wire.write_bytes(&[0xCC, 0xBE]).await;
        let mut data = [0; 9];
        self.wire.read_bytes(&mut data).await;
        match Self::crc8(&data) == 0 {
            true => Ok(((data[1] as u32) << 8 | data[0] as u32) as f32 / 16.),
            false => Err(()),
        }
    }
}