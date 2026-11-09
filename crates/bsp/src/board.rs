//! Board support package for the STM32H735G-DK
use embedded_display_controller::{
    DisplayConfiguration, DisplayController, DisplayControllerLayer, PixelFormat,
};
use stm32h7xx_hal::gpio::{
    gpioc::{PC13, PC2, PC3},
    gpiog::PG15,
    Analog, Input, Output, PinState, PushPull, Speed,
};
use stm32h7xx_hal::ltdc::{Ltdc, LtdcLayer1};
use stm32h7xx_hal::rcc::PllConfigStrategy;
use stm32h7xx_hal::{pac, prelude::*};

pub const LCD_WIDTH: usize = 480;
pub const LCD_HEIGHT: usize = 272;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

pub struct Board {
    pub leds: Leds,
    pub button: Button,
    pub display: Display,
    pub core_clock_hz: u32,
}

impl Board {
    /// Takes the chip's peripherals and sets the board up.
    /// Returns `None` if they were already taken.
    pub fn take() -> Option<Self> {
        Some(Self::init(pac::Peripherals::take()?))
    }

    /// Initialization
    pub fn init(dp: pac::Peripherals) -> Self {
        let power = dp.PWR.constrain().freeze();
        let clocks = dp
            .RCC
            .constrain()
            .pll3_strategy(PllConfigStrategy::Iterative)
            .pll3_r_ck(PIXEL_CLOCK_HZ.Hz())
            .freeze(power, &dp.SYSCFG);

        let gpioa = dp.GPIOA.split(clocks.peripheral.GPIOA);
        let gpiob = dp.GPIOB.split(clocks.peripheral.GPIOB);
        let gpioc = dp.GPIOC.split(clocks.peripheral.GPIOC);
        let gpiod = dp.GPIOD.split(clocks.peripheral.GPIOD);
        let gpioe = dp.GPIOE.split(clocks.peripheral.GPIOE);
        let gpiog = dp.GPIOG.split(clocks.peripheral.GPIOG);
        let gpioh = dp.GPIOH.split(clocks.peripheral.GPIOH);

        /// LTDC pins specific this is here for the sake of clarity and to avoid repetition
        macro_rules! ltdc_pins {
            ($af:literal: $($pin:expr),+ $(,)?) => {
                $( let _ = $pin.into_alternate::<$af>().speed(Speed::VeryHigh); )+
            };
        }
        ltdc_pins!(14: gpioa.pa3, gpioa.pa4, gpioa.pa6);
        ltdc_pins!(13: gpioa.pa8);
        ltdc_pins!(14: gpiob.pb0, gpiob.pb1, gpiob.pb8, gpiob.pb9);
        ltdc_pins!(14: gpioc.pc6, gpioc.pc7);
        ltdc_pins!(14: gpiod.pd0, gpiod.pd3, gpiod.pd6);
        ltdc_pins!(14: gpioe.pe0, gpioe.pe1, gpioe.pe11, gpioe.pe12, gpioe.pe15);
        ltdc_pins!(14: gpiog.pg7, gpiog.pg14);
        ltdc_pins!(14: gpioh.ph3, gpioh.ph8, gpioh.ph9, gpioh.ph10, gpioh.ph11, gpioh.ph15);
        ltdc_pins!(9: gpioh.ph4);

        // Panel specific pins
        let _display_enable = gpioe.pe13.into_push_pull_output_in_state(PinState::Low);
        let _display_control = gpiod.pd10.into_push_pull_output_in_state(PinState::High);
        let backlight = gpiog.pg15.into_push_pull_output_in_state(PinState::Low);
        let mut ltdc = Ltdc::new(dp.LTDC, clocks.peripheral.LTDC, &clocks.clocks);
        ltdc.init(PANEL);
        let display = Display::new(ltdc.split(), backlight);

        // LED and Button initialization
        let leds = Leds::new(gpioc.pc3, gpioc.pc2);
        let button = Button::new(gpioc.pc13);

        Self {
            leds,
            button,
            display,
            core_clock_hz: clocks.clocks.c_ck().raw(),
        }
    }
}

const PIXEL_CLOCK_HZ: u32 = 9_630_000;
const PANEL: DisplayConfiguration = DisplayConfiguration {
    active_width: LCD_WIDTH as u16,
    active_height: LCD_HEIGHT as u16,
    h_back_porch: 13,
    h_front_porch: 32,
    h_sync: 41,
    v_back_porch: 2,
    v_front_porch: 2,
    v_sync: 10,
    h_sync_pol: false,
    v_sync_pol: false,
    not_data_enable_pol: false,
    pixel_clock_pol: true,
};

/// Display object
pub struct Display {
    framebuffer: &'static mut [u16; LCD_PIXELS],
    backlight: PG15<Output<PushPull>>,
    _layer: LtdcLayer1,
}

impl Display {
    /// Points the LTDC at the framebuffer and switches the backlight on.
    fn new(mut layer: LtdcLayer1, mut backlight: PG15<Output<PushPull>>) -> Self {
        let framebuffer = take_framebuffer();
        // SAFETY: the pointer is to the framebuffer, which lives for the whole
        // program and is never moved. The LTDC only reads it.
        #[allow(unsafe_code)]
        unsafe {
            layer.enable(framebuffer.as_ptr(), PixelFormat::RGB565);
        }
        backlight.set_high();
        Self {
            framebuffer,
            backlight,
            _layer: layer,
        }
    }

    /// Helper function to get a mutable slice of the framebuffer
    pub fn framebuffer(&mut self) -> &mut [u16] {
        &mut self.framebuffer[..]
    }

    /// Turns the backlight on or off.
    pub fn set_backlight(&mut self, on: bool) {
        if on {
            self.backlight.set_high();
        } else {
            self.backlight.set_low();
        }
    }
}

/// The frame in RAM. The LTDC reads it to paint the display
#[allow(unsafe_code)]
fn take_framebuffer() -> &'static mut [u16; LCD_PIXELS] {
    static mut FRAMEBUFFER: [u16; LCD_PIXELS] = [0; LCD_PIXELS];
    // SAFETY: only `Display::new` calls this, and `Board::init` runs only once
    // because the chip's peripherals can be taken only once, so this is the
    // only reference to the framebuffer.
    unsafe { &mut *core::ptr::addr_of_mut!(FRAMEBUFFER) }
}

/// LED object 
pub struct Leds {
    green: PC3<Output<PushPull>>,
    red: PC2<Output<PushPull>>,
}

impl Leds {
    /// Configures the pins as outputs. Both LEDs start off, with no flash.
    fn new(green: PC3<Analog>, red: PC2<Analog>) -> Self {
        Self {
            green: green.into_push_pull_output_in_state(PinState::High),
            red: red.into_push_pull_output_in_state(PinState::High),
        }
    }

    /// Turns the LEDs on or off.
    pub fn set(&mut self, green: bool, red: bool) {
        if green {
            self.green.set_low();
        } else {
            self.green.set_high();
        }
        if red {
            self.red.set_low();
        } else {
            self.red.set_high();
        }
    }
}

/// Button object
pub struct Button {
    pin: PC13<Input>,
}

impl Button {
    /// Configures the pin as a pulled-down input.
    fn new(pin: PC13<Analog>) -> Self {
        Self {
            pin: pin.into_pull_down_input(),
        }
    }

    /// True while the button is held down.
    pub fn is_pressed(&self) -> bool {
        self.pin.is_high()
    }
}
