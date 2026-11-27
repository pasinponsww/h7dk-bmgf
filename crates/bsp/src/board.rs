//! Board support package for the STM32H735G-DK
use core::mem::MaybeUninit;

use embedded_display_controller::{
    DisplayConfiguration, DisplayController, DisplayControllerLayer, PixelFormat,
};
use stm32h7xx_hal::gpio::{
    gpioc::{PC13, PC2, PC3},
    gpiog::PG15,
    Analog, Input, Output, PinState, PushPull, Speed,
};
use stm32h7xx_hal::hal::blocking::i2c::{Write, WriteRead};
use stm32h7xx_hal::i2c::{self, I2c};
use stm32h7xx_hal::ltdc::{Ltdc, LtdcLayer1};
use stm32h7xx_hal::rcc::PllConfigStrategy;
use stm32h7xx_hal::rng::RngExt;
use stm32h7xx_hal::{pac, prelude::*};

pub const LCD_WIDTH: usize = 480;
pub const LCD_HEIGHT: usize = 272;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

pub struct Board {
    pub leds: Leds,
    pub button: Button,
    pub display: Display,
    pub touch: Touch,
    pub core_clock_hz: u32,
    /// 32 truly random bits from the chip's hardware random number generator,
    /// different on every start-up. Seed the app's random numbers with it.
    pub seed: u32,
}

impl Board {
    /// Takes the chip's peripherals and sets the board up.
    /// Returns `None` if they were already taken.
    pub fn take() -> Option<Self> {
        Some(Self::init(pac::Peripherals::take()?))
    }

    /// Initialization
    pub fn init(dp: pac::Peripherals) -> Self {
        step("start");
        // Keep the debug link alive while the core sleeps. RTIC's idle loop runs
        // `wfi`, and without these bits the H7 stops the debug clocks in sleep,
        // so probe-rs fails with SwdApFault right after boot.
        dp.DBGMCU.cr.modify(|_, w| {
            w.dbgsleep_d1()
                .set_bit()
                .dbgstop_d1()
                .set_bit()
                .dbgstby_d1()
                .set_bit()
                .d1dbgcken()
                .set_bit()
                .d3dbgcken()
                .set_bit()
        });

        step("debug clocks kept on in sleep");

        step("power: direct SMPS");
        // The STM32H735G-DK powers the core from the on-chip SMPS (as in the
        // HAL's own stm32h735g-dk example). Leaving the supply on its default
        // can hang the HAL waiting for the voltage to settle.
        let power = dp.PWR.constrain().smps().freeze();
        step("power ok; clocks: 200 MHz from PLL1, LCD pixel clock from PLL3");
        let clocks = dp
            .RCC
            .constrain()
            .sys_ck(200.MHz())
            .hclk(200.MHz())
            .pll3_strategy(PllConfigStrategy::Iterative)
            .pll3_r_ck(PIXEL_CLOCK_HZ.Hz())
            .freeze(power, &dp.SYSCFG);
        start_pll3();
        step("clocks ok; gpio ports");
        let gpioa = dp.GPIOA.split(clocks.peripheral.GPIOA);
        let gpiob = dp.GPIOB.split(clocks.peripheral.GPIOB);
        let gpioc = dp.GPIOC.split(clocks.peripheral.GPIOC);
        let gpiod = dp.GPIOD.split(clocks.peripheral.GPIOD);
        let gpioe = dp.GPIOE.split(clocks.peripheral.GPIOE);
        let gpiof = dp.GPIOF.split(clocks.peripheral.GPIOF);
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
        step("lcd pins set");

        // Panel specific pins
        let _display_enable = gpioe.pe13.into_push_pull_output_in_state(PinState::Low);
        let _display_control = gpiod.pd10.into_push_pull_output_in_state(PinState::High);
        let backlight = gpiog.pg15.into_push_pull_output_in_state(PinState::Low);
        step("lcd controller");
        let mut ltdc = Ltdc::new(dp.LTDC, clocks.peripheral.LTDC, &clocks.clocks);
        ltdc.init(PANEL);
        step("lcd layer, framebuffer and backlight");
        let display = Display::new(ltdc.split(), backlight);

        step("leds and button");
        // LED and Button initialization
        let leds = Leds::new(gpioc.pc3, gpioc.pc2);
        let button = Button::new(gpioc.pc13);

        step("touch controller (I2C4)");
        // The touch controller sits on I2C4: SCL on PF14, SDA on PF15.
        let scl = gpiof.pf14.into_alternate_open_drain::<4>();
        let sda = gpiof.pf15.into_alternate_open_drain::<4>();
        let i2c = dp.I2C4.i2c(
            (scl, sda),
            400.kHz(),
            clocks.peripheral.I2C4,
            &clocks.clocks,
        );
        let touch = Touch::new(i2c);

        step("random seed (RNG)");
        let seed = random_seed(dp.RNG, clocks.peripheral.RNG, &clocks.clocks);

        step("board ready");
        Self {
            leds,
            button,
            display,
            touch,
            core_clock_hz: clocks.clocks.c_ck().raw(),
            seed,
        }
    }
}

/// Used if the hardware random number generator reports an error.
const FALLBACK_SEED: u32 = 0xA5A5_1234;

/// Reads 32 random bits from the RNG peripheral, then switches it off again.
/// The RNG runs from HSI48, which the HAL always switches on.
fn random_seed(
    rng: pac::RNG,
    prec: stm32h7xx_hal::rcc::rec::Rng,
    clocks: &stm32h7xx_hal::rcc::CoreClocks,
) -> u32 {
    let mut rng = rng.constrain(prec, clocks);
    let seed = match rng.value() {
        Ok(v) => v,
        Err(_) => {
            defmt::warn!("rng: no random number, using the fixed seed");
            FALLBACK_SEED
        }
    };
    rng.release().cr.modify(|_, w| w.rngen().disabled());
    defmt::info!("rng: seed {=u32:#010x}", seed);
    seed
}

/// Switches PLL3 on, which makes the LCD pixel clock.
///
/// stm32h7xx-hal 0.16 sets up PLL3 when only its R output (the pixel clock)
/// is asked for, but it switches the PLL on only when the P output is asked
/// for. Without PLL3 the LCD controller has no pixel clock, and the first
/// access to its registers stalls the bus, taking the debug link down too.
#[allow(unsafe_code)]
fn start_pll3() {
    // SAFETY: only sets PLL3ON, which the HAL left off; nothing else uses
    // PLL3, and its dividers were already written by the HAL.
    let rcc = unsafe { &*pac::RCC::ptr() };
    if rcc.cr.read().pll3on().bit_is_clear() {
        rcc.cr.modify(|_, w| w.pll3on().set_bit());
        while rcc.cr.read().pll3rdy().bit_is_clear() {}
    }
}

/// Logs one start-up step, so the log shows how far start-up got.
fn step(name: &str) {
    defmt::info!("init: {=str}", name);
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

/// The framebuffer's memory, aligned to a cache line.
#[repr(C, align(32))]
struct Framebuffer([u16; LCD_PIXELS]);

/// The frame in RAM. The LTDC reads it to paint the display.
///
/// It lives in the AXI SRAM (see `memory.x`): the LTDC cannot reach the DTCM,
/// where everything else is. That section is not cleared at start-up, so this
/// clears it before the LTDC reads it.
#[allow(unsafe_code)]
fn take_framebuffer() -> &'static mut [u16; LCD_PIXELS] {
    #[link_section = ".axisram.framebuffer"]
    static mut FRAMEBUFFER: MaybeUninit<Framebuffer> = MaybeUninit::uninit();
    // SAFETY: only `Display::new` calls this, and `Board::init` runs only once
    // because the chip's peripherals can be taken only once, so this is the
    // only reference to the framebuffer. All zero bytes are a valid
    // `[u16; N]`, so it is initialised once it has been cleared.
    unsafe {
        let fb = &mut *core::ptr::addr_of_mut!(FRAMEBUFFER);
        core::ptr::write_bytes(fb.as_mut_ptr(), 0, 1);
        &mut fb.assume_init_mut().0
    }
}

/// I2C address of the FocalTech FT5336 (older revisions of the board).
const FT5336_ADDR: u8 = 0x38;
/// FT5336 register with the number of fingers, followed by the first finger.
const FT5336_STATUS: u8 = 0x02;
/// FT5336 register with the chip id.
const FT5336_CHIP_ID: u8 = 0xA8;

/// While no controller answers, look for one again every this many reads
/// (about half a second at one read per 16 ms tick). Touch controllers boot
/// slower than the MCU, so after a cold power-up the first look finds nothing.
const RETRY_EVERY: u16 = 32;
/// After this many failed reads in a row, look for the controller again.
const MAX_ERRORS: u16 = 30;

/// I2C addresses a Goodix GT911 (newer revisions of the board) may answer on.
const GT911_ADDRS: [u8; 2] = [0x5D, 0x14];
/// GT911 product id: four ASCII bytes, "911" and a zero.
const GT911_PRODUCT_ID: u16 = 0x8140;
/// GT911 configured resolution: x max then y max, little-endian u16 each.
const GT911_RESOLUTION: u16 = 0x8048;
/// GT911 status: bit 7 "new data", bits 0..3 number of fingers. Then the
/// first finger: track id, x (LE u16), y (LE u16), size.
const GT911_STATUS: u16 = 0x814E;

/// One touch as the controller reports it, and where that is on the screen.
#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub struct TouchPoint {
    /// Position as the controller reports it.
    pub raw: (u16, u16),
    /// Position on the 480 x 272 screen.
    pub x: i16,
    /// Position on the 480 x 272 screen.
    pub y: i16,
}

/// Which touch controller the board has.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Controller {
    /// Nothing answered: touch is off, the button still works.
    Missing,
    /// FocalTech FT5336. Its axes are swapped compared with the LCD, as in
    /// ST's own board support code (`TS_SWAP_XY`).
    Ft5336,
    /// Goodix GT911 at this address, reporting up to this x and y.
    Gt911 { addr: u8, max_x: u16, max_y: u16 },
}

/// The capacitive touch panel on I2C4: an FT5336 or a GT911 depending on the
/// board revision, found at start-up.
pub struct Touch {
    i2c: I2c<pac::I2C4>,
    controller: Controller,
    /// The GT911 only reports when it has news, so this is the last state.
    last: Option<TouchPoint>,
    /// Reads since the last look for a controller.
    since_look: u16,
    /// Failed reads in a row.
    errors: u16,
}

fn clamp(v: u32, size: usize) -> i16 {
    i16::try_from(usize::try_from(v).unwrap_or(0).min(size - 1)).unwrap_or(0)
}

impl Touch {
    /// Finds the touch controller on the bus.
    fn new(i2c: I2c<pac::I2C4>) -> Self {
        let mut t = Self {
            i2c,
            controller: Controller::Missing,
            last: None,
            since_look: 0,
            errors: 0,
        };
        t.look();
        if t.controller == Controller::Missing {
            defmt::info!("touch: no controller answered yet, will keep looking");
        }
        t
    }

    /// Looks for a controller and logs what it finds.
    fn look(&mut self) {
        self.since_look = 0;
        self.errors = 0;
        self.last = None;
        self.controller = self.detect();
        match self.controller {
            Controller::Missing => {}
            Controller::Ft5336 => defmt::info!("touch: FT5336 found"),
            Controller::Gt911 { addr, max_x, max_y } => defmt::info!(
                "touch: GT911 found at {=u8:#x}, reports up to ({}, {})",
                addr,
                max_x,
                max_y
            ),
        }
    }

    /// Counts a failed read; after too many in a row, starts looking again.
    fn failed(&mut self) -> Option<TouchPoint> {
        self.errors += 1;
        if self.errors >= MAX_ERRORS {
            defmt::warn!("touch: controller stopped answering, looking for it again");
            self.controller = Controller::Missing;
            self.last = None;
        }
        self.last
    }

    fn detect(&mut self) -> Controller {
        for addr in GT911_ADDRS {
            let mut id = [0u8; 4];
            if self.gt911_read(addr, GT911_PRODUCT_ID, &mut id).is_ok() && id[0] == b'9' {
                let mut r = [0u8; 4];
                let (max_x, max_y) = match self.gt911_read(addr, GT911_RESOLUTION, &mut r) {
                    Ok(()) => (
                        u16::from_le_bytes([r[0], r[1]]),
                        u16::from_le_bytes([r[2], r[3]]),
                    ),
                    Err(_) => (0, 0),
                };
                // Fall back to the screen size if the config looks empty.
                let (max_x, max_y) = if max_x == 0 || max_y == 0 {
                    (LCD_WIDTH as u16, LCD_HEIGHT as u16)
                } else {
                    (max_x, max_y)
                };
                return Controller::Gt911 { addr, max_x, max_y };
            }
        }
        let mut id = [0u8];
        if self
            .i2c
            .write_read(FT5336_ADDR, &[FT5336_CHIP_ID], &mut id)
            .is_ok()
        {
            return Controller::Ft5336;
        }
        Controller::Missing
    }

    fn gt911_read(&mut self, addr: u8, reg: u16, buf: &mut [u8]) -> Result<(), i2c::Error> {
        self.i2c.write_read(addr, &reg.to_be_bytes(), buf)
    }

    fn gt911_write(&mut self, addr: u8, reg: u16, value: u8) -> Result<(), i2c::Error> {
        let [hi, lo] = reg.to_be_bytes();
        self.i2c.write(addr, &[hi, lo, value])
    }

    /// The first finger on the glass, or `None` when nothing touches it (or
    /// there is no controller).
    pub fn read(&mut self) -> Option<TouchPoint> {
        match self.controller {
            Controller::Missing => {
                self.since_look += 1;
                if self.since_look >= RETRY_EVERY {
                    self.look();
                }
                None
            }
            Controller::Ft5336 => self.read_ft5336(),
            Controller::Gt911 { addr, max_x, max_y } => self.read_gt911(addr, max_x, max_y),
        }
    }

    fn read_ft5336(&mut self) -> Option<TouchPoint> {
        let mut b = [0u8; 5];
        if self
            .i2c
            .write_read(FT5336_ADDR, &[FT5336_STATUS], &mut b)
            .is_err()
        {
            return self.failed();
        }
        self.errors = 0;
        let fingers = b[0] & 0x0f;
        if fingers == 0 || fingers > 5 {
            return None;
        }
        let rx = u16::from(b[1] & 0x0f) << 8 | u16::from(b[2]);
        let ry = u16::from(b[3] & 0x0f) << 8 | u16::from(b[4]);
        Some(TouchPoint {
            raw: (rx, ry),
            x: clamp(u32::from(ry), LCD_WIDTH),
            y: clamp(u32::from(rx), LCD_HEIGHT),
        })
    }

    fn read_gt911(&mut self, addr: u8, max_x: u16, max_y: u16) -> Option<TouchPoint> {
        let mut b = [0u8; 6];
        if self.gt911_read(addr, GT911_STATUS, &mut b).is_err() {
            return self.failed();
        }
        self.errors = 0;
        let status = b[0];
        if status & 0x80 == 0 {
            // No new report since the last read: nothing changed.
            return self.last;
        }
        // Tell the controller the report was read, so it sends the next one.
        let _ = self.gt911_write(addr, GT911_STATUS, 0);
        let fingers = status & 0x0f;
        self.last = if fingers == 0 || fingers > 5 {
            None
        } else {
            let rx = u16::from_le_bytes([b[2], b[3]]);
            let ry = u16::from_le_bytes([b[4], b[5]]);
            // Scale the controller's range onto the screen, turning it if the
            // controller is set up in portrait.
            let (px, py, wx, wy) = if max_x >= max_y {
                (rx, ry, max_x, max_y)
            } else {
                (ry, rx, max_y, max_x)
            };
            let scale = |v: u16, range: u16, size: usize| {
                u32::from(v) * u32::try_from(size).unwrap_or(0) / u32::from(range.max(1))
            };
            Some(TouchPoint {
                raw: (rx, ry),
                x: clamp(scale(px, wx, LCD_WIDTH), LCD_WIDTH),
                y: clamp(scale(py, wy, LCD_HEIGHT), LCD_HEIGHT),
            })
        };
        self.last
    }
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
