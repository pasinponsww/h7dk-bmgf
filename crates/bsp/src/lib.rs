#![no_std]

pub mod board;
pub mod cache;

pub use board::{
    Board, Button, Display, LedTimer, Leds, Touch, TouchPoint, Watchdog, LCD_HEIGHT, LCD_PIXELS,
    LCD_WIDTH, LED_PWM_HZ, SCAN_ACTIVE_START, SCAN_LINES, SCAN_LINE_NS, WATCHDOG_MS,
};
pub use stm32h7xx_hal as hal;
