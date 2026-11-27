#![no_std]

pub mod board;

pub use board::{
    Board, Button, Display, Leds, Touch, TouchPoint, Watchdog, LCD_HEIGHT, LCD_PIXELS, LCD_WIDTH,
    WATCHDOG_MS,
};
pub use stm32h7xx_hal as hal;
