#![no_std]
#![no_main]

pub mod adjustment;
pub mod controls;
pub mod display;
pub mod sensor;
pub mod sh1107;
pub mod sh1107new;

pub const MIN_SUPPORTED_TEMP: f32 = 11.0; // Minimum selectable temperature
pub const MAX_SUPPORTED_TEMP: f32 = 27.0; // Maximum selectable temperature
pub const TEMP_CHECK_INTERVAL: u32 = 300; // Temperature check interval (seconds)
pub const NO_DEVICE_CHECK_INTERVAL: u8 = 60; // Check interval for when no temperature sensor was detected previously (seconds)
pub const DISPLAY_TIMEOUT: u8 = 30; // Turn off display to avoid burn-in
pub const TOLERANCE: f32 = 0.25; // Allowable variance on either side of the target

pub const TERM_KP: f32 = 10.0; // Proportional term - Basic steering (This is the first parameter you should tune for a particular setup)
pub const TERM_KI: f32 = 0.01; // Integral term - Compensate for heat loss by vessel
pub const TERM_KD: f32 = 150.0; //
