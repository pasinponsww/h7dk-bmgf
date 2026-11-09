
#![no_std]
#![warn(missing_docs)]

pub mod board;

pub use board::{Board, Button, Display, Leds, LCD_HEIGHT, LCD_PIXELS, LCD_WIDTH};
pub use stm32h7xx_hal as hal;
