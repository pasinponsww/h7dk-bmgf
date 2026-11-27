#![no_std]

pub mod board;
pub mod cache;
pub mod clock;
pub mod cordic;
pub mod flash;
pub mod hyperram;
pub mod storage;
pub mod thermo;

pub use board::{
    Board, Button, Display, LedTimer, Leds, Touch, TouchPoint, Watchdog, LCD_HEIGHT, LCD_PIXELS,
    LCD_WIDTH, LED_PWM_HZ, SCAN_ACTIVE_START, SCAN_LINES, SCAN_LINE_NS, STICKER_PIXELS,
    STICKER_SIZE, WATCHDOG_MS,
};
pub use stm32h7xx_hal as hal;
