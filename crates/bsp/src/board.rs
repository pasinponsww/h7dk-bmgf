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
use stm32h7xx_hal::independent_watchdog::IndependentWatchdog;
use stm32h7xx_hal::ltdc::{Ltdc, LtdcLayer1};
use stm32h7xx_hal::rcc::{PllConfigStrategy, ResetReason};
use stm32h7xx_hal::rng::RngExt;
use stm32h7xx_hal::timer::{Event as TimerEvent, Timer};
use stm32h7xx_hal::{pac, prelude::*};

use crate::aes::Aes;
use crate::clock::Clock;
use crate::cordic::Cordic;
use crate::flash::ExternalFlash;
use crate::hyperram::HyperRamPort;
use crate::storage::SaveSector;
use crate::thermo::Thermometer;

pub const LCD_WIDTH: usize = 480;
pub const LCD_HEIGHT: usize = 272;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;
/// Width and height of the sticker on the LCD controller's second layer.
pub const STICKER_SIZE: u16 = 48;
/// Pixels in the sticker (ARGB4444, two bytes each).
pub const STICKER_PIXELS: usize = STICKER_SIZE as usize * STICKER_SIZE as usize;

pub struct Board {
    pub leds: Leds,
    /// Ticks at [`LED_PWM_HZ`] to switch the LEDs on and off for dimming.
    pub led_timer: LedTimer,
    pub button: Button,
    pub display: Display,
    pub touch: Touch,
    pub watchdog: Watchdog,
    /// The 64 MB flash on OCTOSPI1, for reading the photo album.
    pub flash: ExternalFlash,
    /// The 16 MB HyperRAM on OCTOSPI2, off until it is started.
    pub hyperram: HyperRamPort,
    /// The real-time clock.
    pub clock: Clock,
    /// The flash sector that keeps saved progress.
    pub save: SaveSector,
    /// The CORDIC co-processor, for sine and cosine.
    pub cordic: Cordic,
    /// The chip's temperature sensor.
    pub thermometer: Thermometer,
    /// The AES engine.
    pub aes: Aes,
    /// The last restart was the watchdog's doing: something hung.
    pub reset_by_watchdog: bool,
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

        // Pause the watchdog while the debugger has the core halted, so
        // stepping through code does not restart the board.
        dp.DBGMCU.apb4fz1.modify(|_, w| w.dbg_wdglsd1().set_bit());

        step("debug clocks kept on in sleep, watchdog paused by the debugger");

        step("power: direct SMPS");
        // The STM32H735G-DK powers the core from the on-chip SMPS (as in the
        // HAL's own stm32h735g-dk example). Leaving the supply on its default
        // can hang the HAL waiting for the voltage to settle.
        let mut power = dp.PWR.constrain().smps().freeze();
        let backup = power.backup();
        step("power ok; clocks: 400 MHz core from PLL1, LCD pixel clock from PLL3");
        let mut rcc = dp.RCC.constrain();
        let reset_by_watchdog = matches!(
            rcc.get_reset_reason(),
            ResetReason::IndependentWatchdogReset | ResetReason::GenericWatchdogReset
        );
        if reset_by_watchdog {
            defmt::warn!("init: the watchdog restarted the board (something hung)");
        }
        let clocks = rcc
            // 400 MHz is the fastest the core may run at the default voltage
            // (VOS1). The buses stay at their VOS1 limit of 200 MHz, so the
            // peripherals see the same clocks as before.
            .sys_ck(CORE_HZ.Hz())
            .hclk(200.MHz())
            .pll3_strategy(PllConfigStrategy::Iterative)
            .pll3_r_ck(PIXEL_CLOCK_HZ.Hz())
            .freeze(power, &dp.SYSCFG);
        start_pll3();
        // The ADCs' kernel clock from per_ck (HSI, 64 MHz): PLL2, their
        // default, is not used. Set before any peripheral is taken out.
        let mut clocks = clocks;
        clocks
            .peripheral
            .kernel_adc_clk_mux(stm32h7xx_hal::rcc::rec::AdcClkSel::Per);
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
        let led_timer = LedTimer::new(dp.TIM2.timer(
            LED_PWM_HZ.Hz(),
            clocks.peripheral.TIM2,
            &clocks.clocks,
        ));
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

        step("external flash (OCTOSPI1, 1-bit SPI at 50 MHz)");
        // The pins as wired on the STM32H735G-DK (and in the HAL's own
        // octospi example for this board).
        macro_rules! ospi_pins {
            ($af:literal: $($pin:expr),+ $(,)?) => {
                $( let _ = $pin.into_alternate::<$af>().speed(Speed::VeryHigh); )+
            };
        }
        ospi_pins!(9: gpiof.pf10, gpiod.pd11, gpiod.pd12, gpioe.pe2, gpiod.pd13, gpiog.pg9);
        ospi_pins!(10: gpiog.pg6, gpiod.pd4, gpiod.pd5, gpiod.pd7);
        let ospi =
            dp.OCTOSPI1
                .octospi_unchecked(50.MHz(), &clocks.clocks, clocks.peripheral.OCTOSPI1);
        let mut flash = ExternalFlash::new(ospi);
        match flash.id() {
            Some(id) if flash.present() => defmt::info!("flash: MX25LM51245G, id {=[u8]:02x}", id),
            Some(id) => defmt::warn!("flash: unexpected id {=[u8]:02x}", id),
            None => defmt::warn!("flash: no answer"),
        }

        step("hyperram pins (OCTOSPI2), started later if needed");
        // As in the HAL's octospi_hyperram example for this board.
        macro_rules! hyperram_pins {
            ($af:literal: $($pin:expr),+ $(,)?) => {
                $( let _ = $pin.into_alternate::<$af>().speed(Speed::High).internal_pull_up(true); )+
            };
        }
        hyperram_pins!(9: gpiof.pf12, gpiof.pf4, gpiof.pf0, gpiof.pf1, gpiof.pf2, gpiof.pf3);
        hyperram_pins!(9: gpiog.pg0, gpiog.pg1, gpiog.pg11);
        hyperram_pins!(3: gpiog.pg12, gpiog.pg10);
        let hyperram = HyperRamPort {
            ospi: dp.OCTOSPI2,
            rec: clocks.peripheral.OCTOSPI2,
            clocks: clocks.clocks,
        };

        step("real-time clock");
        let clock = Clock::start(
            dp.RTC,
            backup.expect("the backup domain is taken only here"),
            &clocks.clocks,
        );

        step("random seed (RNG)");
        let seed = random_seed(dp.RNG, clocks.peripheral.RNG, &clocks.clocks);

        step("watchdog");
        let watchdog = Watchdog::start(dp.IWDG1);

        step("board ready");
        Self {
            leds,
            led_timer,
            button,
            display,
            touch,
            watchdog,
            flash,
            hyperram,
            clock,
            save: SaveSector::new(dp.FLASH),
            cordic: Cordic::new(dp.CORDIC, clocks.peripheral.CORDIC),
            thermometer: Thermometer::new(dp.ADC3, clocks.peripheral.ADC3, &clocks.clocks),
            aes: Aes::new(dp.CRYP, clocks.peripheral.CRYPT),
            reset_by_watchdog,
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

/// Core clock. 400 MHz is the VOS1 limit for the STM32H735 (RM0468); going
/// to 520 MHz would need the higher VOS0 core voltage.
pub const CORE_HZ: u32 = 400_000_000;

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

/// Values the LCD controller's line counter (LTDC_CPSR.CYPOS) runs through
/// in one refresh: sync, back porch, the visible rows and the front porch.
pub const SCAN_LINES: u16 =
    PANEL.v_sync + PANEL.v_back_porch + PANEL.active_height + PANEL.v_front_porch;
/// The counter value of the first visible row.
pub const SCAN_ACTIVE_START: u16 = PANEL.v_sync + PANEL.v_back_porch;
/// How long one line takes, in nanoseconds.
pub const SCAN_LINE_NS: u32 = ((PANEL.h_sync as u64
    + PANEL.h_back_porch as u64
    + PANEL.active_width as u64
    + PANEL.h_front_porch as u64)
    * 1_000_000_000
    / PIXEL_CLOCK_HZ as u64) as u32;

/// Display object
pub struct Display {
    framebuffer: &'static mut [u16; LCD_PIXELS],
    /// The sticker's pixels, which layer 2 shows.
    sticker: &'static mut [u16; STICKER_PIXELS],
    /// Where the sticker is (top-left), or `None` while it is hidden.
    sticker_at: Option<(u16, u16)>,
    backlight: PG15<Output<PushPull>>,
    _layer: LtdcLayer1,
    /// The layer's constant alpha, as last set.
    opacity: u8,
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
        let sticker = take_sticker();
        set_up_sticker_layer(sticker);
        Self {
            framebuffer,
            sticker,
            sticker_at: None,
            backlight,
            _layer: layer,
            opacity: u8::MAX,
        }
    }

    /// The sticker's pixels, ARGB4444, row by row. Draw it once at start-up;
    /// the LCD controller reads it from then on.
    pub fn sticker_pixels(&mut self) -> &mut [u16; STICKER_PIXELS] {
        self.sticker
    }

    /// Shows the sticker with its top-left corner at `at` (kept fully on the
    /// screen), or hides it. The LCD controller lays it over the picture
    /// itself and takes the change at the next vertical blank, so moving it
    /// costs no drawing.
    #[allow(unsafe_code)]
    pub fn place_sticker(&mut self, at: Option<(i16, i16)>) {
        let side = STICKER_SIZE;
        let clamp = |v: i16, room: usize| {
            let max = u16::try_from(room).unwrap_or(u16::MAX) - side;
            u16::try_from(v.max(0)).unwrap_or(0).min(max)
        };
        let at = at.map(|(x, y)| (clamp(x, LCD_WIDTH), clamp(y, LCD_HEIGHT)));
        if at == self.sticker_at {
            return;
        }
        self.sticker_at = at;
        // SAFETY: only `Display` writes layer 2's registers, and a `Display`
        // exists only once the LTDC is set up. The window stays inside the
        // visible area, as the controller needs.
        unsafe {
            let ltdc = &*pac::LTDC::ptr();
            let layer = &ltdc.layer2;
            match at {
                None => layer.cr.modify(|_, w| w.len().clear_bit()),
                Some((x, y)) => {
                    let bp = ltdc.bpcr.read();
                    let (h, v) = (bp.ahbp().bits(), bp.avbp().bits());
                    layer
                        .whpcr
                        .write(|w| w.whstpos().bits(x + h + 1).whsppos().bits(x + side + h));
                    layer
                        .wvpcr
                        .write(|w| w.wvstpos().bits(y + v + 1).wvsppos().bits(y + side + v));
                    layer.cr.modify(|_, w| w.len().set_bit());
                }
            }
            ltdc.srcr.write(|w| w.vbr().set_bit());
        }
    }

    /// Sets the color the picture fades to: the LCD controller's background
    /// color, which shows through wherever the layer is see-through.
    #[allow(unsafe_code)]
    pub fn set_fade_color(&mut self, [r, g, b]: [u8; 3]) {
        // SAFETY: only `Display` writes the LTDC's background color, and a
        // `Display` exists only once the LTDC is set up.
        unsafe {
            (*pac::LTDC::ptr())
                .bccr
                .write(|w| w.bcred().bits(r).bcgreen().bits(g).bcblue().bits(b));
        }
    }

    /// How much of the picture shows: 255 all of it, 0 only the fade color
    /// (see [`Display::set_fade_color`]). The LCD controller blends the two
    /// itself and takes the new value at the next vertical blank, so a change
    /// never lands in the middle of a refresh.
    #[allow(unsafe_code)]
    pub fn set_opacity(&mut self, alpha: u8) {
        if alpha == self.opacity {
            return;
        }
        self.opacity = alpha;
        // SAFETY: only `Display` writes layer 1's constant alpha and asks for
        // a reload, and a `Display` exists only once the LTDC is set up.
        unsafe {
            let ltdc = &*pac::LTDC::ptr();
            ltdc.layer1.cacr.write(|w| w.consta().bits(alpha));
            ltdc.srcr.write(|w| w.vbr().set_bit());
        }
    }

    /// Helper function to get a mutable slice of the framebuffer
    pub fn framebuffer(&mut self) -> &mut [u16] {
        &mut self.framebuffer[..]
    }

    /// The line the LCD controller is reading right now, counting from the
    /// start of the sync lines (see [`SCAN_LINES`] and [`SCAN_ACTIVE_START`]).
    #[allow(unsafe_code)]
    pub fn scan_line() -> u16 {
        // SAFETY: reads a read-only status register. The LTDC is clocked and
        // set up before a `Display` exists, which is the only way to reach this.
        unsafe { (*pac::LTDC::ptr()).cpsr.read().cypos().bits() }
    }

    /// Turns the backlight on or off.
    pub fn set_backlight(&mut self, on: bool) {
        if on == self.backlight.is_set_high() {
            return;
        }
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

/// The sticker's memory, aligned to a cache line.
#[repr(C, align(32))]
struct Sticker([u16; STICKER_PIXELS]);

/// The sticker in RAM. Like the framebuffer it lives in the AXI SRAM, where
/// the LTDC can read it, and starts out cleared (fully see-through).
#[allow(unsafe_code)]
fn take_sticker() -> &'static mut [u16; STICKER_PIXELS] {
    #[link_section = ".axisram.sticker"]
    static mut STICKER: MaybeUninit<Sticker> = MaybeUninit::uninit();
    // SAFETY: only `Display::new` calls this, once (see `take_framebuffer`),
    // so this is the only reference. All zero bytes are a valid `[u16; N]`.
    unsafe {
        let s = &mut *core::ptr::addr_of_mut!(STICKER);
        core::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
        &mut s.assume_init_mut().0
    }
}

/// Sets layer 2 up to show `sticker` (ARGB4444, blended over layer 1 by its
/// own alpha), switched off until [`Display::place_sticker`] shows it.
#[allow(unsafe_code)]
fn set_up_sticker_layer(sticker: &[u16; STICKER_PIXELS]) {
    let line = STICKER_SIZE * 2;
    // SAFETY: runs once from `Display::new`, after the LTDC is set up and
    // before anything else touches layer 2. The buffer is a static in the AXI
    // SRAM, which the LTDC can read, and is never moved.
    unsafe {
        let ltdc = &*pac::LTDC::ptr();
        let layer = &ltdc.layer2;
        layer.cr.modify(|_, w| w.len().clear_bit());
        layer.pfcr.write(|w| w.pf().argb4444());
        layer.cacr.write(|w| w.consta().bits(u8::MAX));
        layer.dccr.reset();
        layer.bfcr.write(|w| w.bf1().pixel().bf2().pixel());
        layer
            .cfbar
            .write(|w| w.cfbadd().bits(sticker.as_ptr() as u32));
        layer
            .cfblr
            .write(|w| w.cfbp().bits(line).cfbll().bits(line + 7));
        layer.cfblnr.write(|w| w.cfblnbr().bits(STICKER_SIZE));
        ltdc.srcr.write(|w| w.imr().set_bit());
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

/// Restarts the board unless it is fed at least this often.
pub const WATCHDOG_MS: u32 = 2_000;

/// The independent watchdog. It runs from its own clock, so it still restarts
/// the board if the code hangs, faults or locks up the bus. Once started it
/// cannot be stopped.
pub struct Watchdog(IndependentWatchdog);

impl Watchdog {
    fn start(iwdg: pac::IWDG1) -> Self {
        let mut w = IndependentWatchdog::new(iwdg);
        w.start(WATCHDOG_MS.millis());
        Self(w)
    }

    /// Tells the watchdog everything is fine for another [`WATCHDOG_MS`].
    pub fn feed(&mut self) {
        self.0.feed();
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

/// How often the LED timer interrupt (TIM2) fires. The LED pins have no
/// hardware PWM, so dimming is done by switching them on every tick.
pub const LED_PWM_HZ: u32 = 10_000;

/// TIM2, set up to raise its interrupt [`LED_PWM_HZ`] times a second.
pub struct LedTimer(Timer<pac::TIM2>);

impl LedTimer {
    fn new(mut timer: Timer<pac::TIM2>) -> Self {
        timer.listen(TimerEvent::TimeOut);
        Self(timer)
    }

    /// Clears the interrupt. Call it first thing in the TIM2 handler.
    pub fn ack(&mut self) {
        self.0.clear_irq();
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
