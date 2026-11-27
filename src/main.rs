#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use defmt_rtt as _;
use panic_probe as _;

use app::pacing::Timing;
use app::state::{NEXT_BUTTON, RESTART_BUTTON};
use app::story::Content;
use app::{Model, Point, Rect, Scene, Story};
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1_000);

/// How often the logic gets a tick and the touch panel is read, in milliseconds.
const TICK_MS: u32 = 16;
/// Holding the blue button this long goes back to the beginning.
const LONG_PRESS_MS: u32 = 1_000;
/// How often the screen is drawn again, in milliseconds (about 30 frames a
/// second), if the LCD controller's line counter cannot be followed.
const FRAME_MS: u32 = 33;
/// After a frame starts, the render task sleeps this long and then waits for
/// the top of the next LCD refresh. A refresh takes 16.8 ms, so frames start
/// on every second refresh: 29.7 frames a second.
const FRAME_LEAD_MS: u32 = 30;
// The sticker the board's second LCD layer shows is the one `ui` draws.
const _: () = assert!(bsp::STICKER_PIXELS == ui::sticker::PIXELS);

/// While the screen sleeps, the render task looks this often for a wake-up.
const SLEEP_POLL_MS: u32 = 100;

/// The LCD controller's line counter on this panel.
const SCAN: Timing = Timing {
    active_start: bsp::SCAN_ACTIVE_START,
    lines: bsp::SCAN_LINES,
    line_ns: bsp::SCAN_LINE_NS,
};
/// Rows drawn at a time. Each finished band is copied into the framebuffer.
const BAND_ROWS: usize = 16;
/// Pixels in one band.
const BAND_LEN: usize = bsp::LCD_WIDTH * BAND_ROWS;

fn center(r: Rect) -> Point {
    Point::new(r.x + r.w / 2, r.y + r.h / 2)
}

/// The tap that moves the story one step forward. A short press of the blue
/// button is a shortcut for the touchscreen: it taps whatever is next (the
/// intro, the next object, the Next button, YES, or Start again).
fn next_tap(model: &Model, story: &Content) -> Point {
    match model.scene {
        Scene::Intro => Point::new(240, 136),
        Scene::Story { chapter, found, .. } => {
            let ch = usize::from(chapter);
            (0..story.hotspot_count(ch))
                .find(|i| found & (1u8 << i) == 0)
                .map_or(center(NEXT_BUTTON), |i| center(story.hotspot_rect(ch, i)))
        }
        // The next dot of the heart, or Next once it has unlocked.
        Scene::Trace { hit } => match app::trace::first_missing(hit) {
            Some(i) if !model.next_unlocked(story) => {
                let (x, y) = app::trace::DOT_AT[usize::from(i)];
                Point::new(x, y)
            }
            _ => center(NEXT_BUTTON),
        },
        Scene::Question => center(model.yes_rect()),
        Scene::Celebration => center(RESTART_BUTTON),
    }
}

fn log_scene(model: &Model) {
    match model.scene {
        Scene::Intro => defmt::info!("scene: intro"),
        Scene::Story { chapter, found, .. } => {
            defmt::info!(
                "scene: story, chapter {}, found mask {=u8:b}",
                chapter + 1,
                found
            );
        }
        Scene::Trace { hit } => defmt::info!(
            "scene: trace the heart, {} dots",
            (hit & app::trace::ALL).count_ones()
        ),
        Scene::Question => defmt::info!("scene: question"),
        Scene::Celebration => defmt::info!("scene: celebration"),
    }
}

/// Frames the render task has finished. The tick task reads it to decide
/// whether to feed the watchdog (see `app::liveness`).
static FRAMES: AtomicU32 = AtomicU32::new(0);

/// The LED duty the tick task wants, green in the high byte and red in the
/// low byte. The LED interrupt reads it 10 000 times a second.
static LED_LEVELS: AtomicU16 = AtomicU16::new(0);

fn now_ms() -> u32 {
    Mono::now().duration_since_epoch().to_millis()
}

/// Writes `progress` into the next empty slot of the save sector, erasing it
/// first when every slot is used.
fn save_progress(
    sector: &mut bsp::storage::SaveSector,
    saved: &mut app::save::Found,
    progress: app::save::Progress,
    watchdog: &mut bsp::Watchdog,
) {
    let slot = match saved.free {
        Some(slot) => slot,
        None => {
            // About a second with the CPU waiting on the flash.
            watchdog.feed();
            if sector.erase().is_err() {
                defmt::warn!("save: could not erase the sector");
                return;
            }
            watchdog.feed();
            0
        }
    };
    let seq = saved.seq.wrapping_add(1);
    if sector.write(slot, &progress.encode(seq)).is_err() {
        defmt::warn!("save: could not write slot {}", slot);
        return;
    }
    let next = slot + 1;
    *saved = app::save::Found {
        progress,
        seq,
        free: (next < bsp::storage::SLOTS).then_some(next),
    };
}

/// Sine and cosine from the CORDIC co-processor, for the painter.
struct HwTrig<'a>(&'a mut bsp::cordic::Cordic);

impl ui::trig::Trig for HwTrig<'_> {
    fn sin_cos(&mut self, angle: u16) -> (i16, i16) {
        self.0.sin_cos(angle)
    }
}

/// AES from the chip's CRYP engine, for opening the secret message.
struct HwAes<'a>(&'a mut bsp::aes::Aes);

impl app::secret::Cipher for HwAes<'_> {
    fn ctr(&mut self, key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) {
        self.0.ctr(key, iv, data);
    }
}

/// Opens the story's secret message with the AES engine, or in software if
/// the engine's answer does not check out.
fn open_secret(aes: &mut bsp::aes::Aes) -> Option<app::secret::Opened> {
    use app::secret::{open, SoftAes};
    let sealed = app::story::SECRET?;
    if let Some(opened) = open(&sealed, &mut HwAes(aes)) {
        defmt::info!("secret: opened with the AES engine");
        return Some(opened);
    }
    defmt::warn!("secret: the AES engine's answer did not check out, opening in software");
    open(&sealed, &mut SoftAes)
}

/// The clock's moment in the app's calendar types.
fn moment_of(t: bsp::clock::Moment) -> app::calendar::DateTime {
    app::calendar::DateTime {
        date: app::calendar::Date {
            year: t.year,
            month: t.month,
            day: t.day,
        },
        hour: t.hour,
        minute: t.minute,
        second: t.second,
    }
}

/// Reads the photo album off the external flash into the HyperRAM, if the
/// flash has one and the HyperRAM works. Returns the photos, one after
/// another, and how many there are. The HyperRAM is only switched on when
/// there is an album to put in it.
fn load_album(
    flash: &mut bsp::flash::ExternalFlash,
    hyperram: bsp::hyperram::HyperRamPort,
    watchdog: &mut bsp::Watchdog,
) -> Option<(&'static [u8], u16)> {
    use app::album::{Album, HEADER_BYTES, PHOTO_BYTES};
    let mut header = [0u8; HEADER_BYTES as usize];
    let found = flash
        .read(0, &mut header)
        .ok()
        .and_then(|()| Album::parse(&header));
    let Some(album) = found else {
        defmt::info!("album: none on the flash");
        return None;
    };
    defmt::info!("album: {} photos on the flash", album.count);
    let Some(ram) = hyperram.start() else {
        defmt::warn!("hyperram: failed its check, no slideshow");
        return None;
    };
    defmt::info!("hyperram: 16 MB ok");
    let bytes = usize::try_from(album.bytes()).ok()?;
    let photo = PHOTO_BYTES as usize;
    let started = now_ms();
    for (i, slot) in ram[..bytes].chunks_mut(photo).enumerate() {
        let at = Album::offset(u32::try_from(i).ok()?);
        if flash.read(at, slot).is_err() {
            defmt::warn!("album: could not read photo {}", i + 1);
            return None;
        }
        // Half a second for a full album: keep the watchdog happy.
        watchdog.feed();
    }
    defmt::info!(
        "album: {} photos loaded in {} ms",
        album.count,
        now_ms().wrapping_sub(started)
    );
    let count = u16::try_from(album.count).ok()?;
    Some((&ram[..bytes], count))
}

/// Waits for the blank lines at the top of the next LCD refresh: sleeps
/// through most of the wait, then watches the line counter. Returns false if
/// the counter never got there within two refreshes (the LCD is not running).
async fn wait_for_top_blank() -> bool {
    let start = now_ms();
    let give_up_ms = 2 * SCAN.refresh_us() / 1_000 + 5;
    loop {
        let line = bsp::Display::scan_line();
        if SCAN.in_top_blank(line) {
            return true;
        }
        if now_ms().wrapping_sub(start) > give_up_ms {
            return false;
        }
        let us = SCAN.us_until_top(line);
        if us > 1_500 {
            Mono::delay((us / 1_000 - 1).millis()).await;
        }
    }
}

#[rtic::app(device = bsp::hal::pac, dispatchers = [SPI1, SPI2])]
mod firmware {
    use app::frame_stats::FrameStats;
    use app::gesture::{Gesture, Swipe};
    use app::leds::Dither;
    use app::liveness::Liveness;
    use app::pacing::Racer;
    use app::screen::{Cues, Painter};
    use app::state::Power;
    use app::story::Content;
    use app::{update, Effects, Event, Model, Point, TouchEvent, XorShift32};
    use bsp::{Board, Button, Display, LedTimer, Leds, Touch, Watchdog};
    use cortex_m::peripheral::DWT;
    use rtic_monotonics::systick::prelude::*;
    use ui::frame::Band;

    use super::{
        load_album, log_scene, moment_of, next_tap, now_ms, open_secret, save_progress,
        wait_for_top_blank, HwTrig, Mono, Ordering, BAND_LEN, BAND_ROWS, FRAMES, FRAME_LEAD_MS,
        FRAME_MS, LED_LEVELS, LONG_PRESS_MS, SCAN, SLEEP_POLL_MS, TICK_MS,
    };

    #[shared]
    struct Shared {
        model: Model,
        cues: Cues,
    }

    #[local]
    struct Local {
        thermometer: bsp::thermo::Thermometer,
        aes: bsp::aes::Aes,
        painter: Painter,
        cordic: bsp::cordic::Cordic,
        save: bsp::storage::SaveSector,
        saved: app::save::Found,
        clock: bsp::clock::Clock,
        leds: Leds,
        led_timer: LedTimer,
        button: Button,
        touch: Touch,
        display: Display,
        stats: FrameStats,
        /// The longest a band waits for the LCD controller, in CPU cycles
        /// (one refresh).
        max_wait: u32,
        watchdog: Watchdog,
        liveness: Liveness,
        rng: XorShift32,
        /// When the blue button went down, while it is held.
        pressed_since: Option<u32>,
        /// The current press was long and has already restarted the story.
        long_press_done: bool,
        was_touching: bool,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("h7dk-fw boot");
        let mut core = cx.core;
        // Both caches. The framebuffer's memory is kept out of the data
        // cache, so new pixels always reach the LCD controller.
        bsp::cache::enable(&mut core.SCB, &mut core.CPUID, &mut core.MPU);

        let Board {
            leds,
            led_timer,
            button,
            mut display,
            touch,
            mut watchdog,
            mut flash,
            hyperram,
            mut clock,
            save,
            cordic,
            thermometer,
            aes,
            reset_by_watchdog,
            core_clock_hz,
            seed,
        } = Board::init(cx.device);
        Mono::start(core.SYST, core_clock_hz);

        // A photo album written by `cargo proj mbedded photos`, if there is
        // one, copied into the HyperRAM for the celebration's slideshow.
        let (painter, album_count) = match load_album(&mut flash, hyperram, &mut watchdog) {
            Some((photos, count)) => (Painter::with_album(photos), count),
            None => (Painter::new(), 0),
        };
        if reset_by_watchdog {
            defmt::warn!("started again after a hang");
        }
        defmt::info!("core clock {} Hz", core_clock_hz);

        // The sticker for the LCD controller's second layer, drawn once.
        ui::sticker::render(display.sticker_pixels());
        // Start from plain pink: the intro fades in over it, and the empty
        // framebuffer is never seen.
        display.set_fade_color(ui::theme::BACKGROUND_RGB);
        display.set_opacity(0);

        // The cycle counter times each frame for the once-a-second log line.
        core.DCB.enable_trace();
        DWT::unlock();
        core.DWT.enable_cycle_counter();

        // The real-time clock: set it from the time the firmware was built if
        // it has lost the time or is behind it.
        if let Some(built) = app::calendar::BUILT_AT {
            let behind = clock.now().is_none_or(|now| moment_of(now) < built);
            if behind {
                clock.set(bsp::clock::Moment {
                    year: built.date.year,
                    month: built.date.month,
                    day: built.date.day,
                    hour: built.hour,
                    minute: built.minute,
                    second: built.second,
                });
                defmt::info!("clock: set from the build time");
            }
        }
        match clock.now() {
            Some(t) => defmt::info!("clock: {}", t),
            None => defmt::info!("clock: not set (build with cargo proj to set it)"),
        }

        // Saved progress: where the story was, and how many yeses.
        let found = app::save::scan(save.bytes());
        defmt::info!(
            "save: {} yes so far, slot {} free",
            found.progress.yes_count,
            found.free.unwrap_or(usize::MAX)
        );

        let mut model = Model::new(0);
        model.restore(found.progress);
        model.album = album_count;
        model.today = clock.now().map(|t| moment_of(t).date);
        log_scene(&model);

        tick::spawn().ok();
        render::spawn().ok();

        (
            Shared {
                model,
                cues: Cues::new(),
            },
            Local {
                thermometer,
                aes,
                cordic,
                save,
                saved: found,
                clock,
                leds,
                led_timer,
                button,
                touch,
                display,
                painter,
                stats: FrameStats::new(core_clock_hz, 0),
                max_wait: SCAN.refresh_us() * (core_clock_hz / 1_000_000),
                watchdog,
                liveness: Liveness::new(0),
                // A new seed from the hardware RNG on every start-up, so the
                // NO button runs a different way each time.
                rng: XorShift32::new(seed),
                pressed_since: None,
                long_press_done: false,
                was_touching: false,
            },
        )
    }

    /// Runs the logic: the clock, the touch panel and the blue button.
    #[task(priority = 2, shared = [model, cues], local = [button, touch, watchdog, liveness, rng, pressed_since, long_press_done, was_touching, clock, save, saved, thermometer, gesture: Gesture = Gesture::new(), last_date_read: u32 = 0])]
    async fn tick(mut cx: tick::Context) {
        let story = Content;
        let mut next = Mono::now();
        loop {
            next += TICK_MS.millis();
            Mono::delay_until(next).await;
            let now = now_ms();

            // Today's date, read from the real-time clock once a second.
            if now.wrapping_sub(*cx.local.last_date_read) >= 1_000 {
                *cx.local.last_date_read = now;
                let today = cx.local.clock.now().map(|t| moment_of(t).date);
                // And how warm the chip is.
                let (cal30, cal110) = bsp::thermo::Thermometer::calibration();
                let celsius = cx
                    .local
                    .thermometer
                    .read()
                    .and_then(|raw| app::warmth::celsius(raw, cal30, cal110));
                cx.shared.model.lock(|m| {
                    m.today = today;
                    if m.celsius.is_none() {
                        defmt::info!("chip temperature: {} C", celsius.unwrap_or(i16::MIN));
                    }
                    m.celsius = celsius;
                });
            }

            // Feed the watchdog only while the screen keeps drawing too.
            if cx.local.liveness.alive(FRAMES.load(Ordering::Relaxed), now) {
                cx.local.watchdog.feed();
            }

            // A finger that just touched the glass is a tap.
            let finger = cx.local.touch.read();
            let finger_down = finger.is_some() && !*cx.local.was_touching;
            // A finger that stays on the glass is moving (tracing the heart).
            let finger_held = finger.is_some() && *cx.local.was_touching;
            let finger_up = finger.is_none() && *cx.local.was_touching;
            *cx.local.was_touching = finger.is_some();
            // A quick sideways move before the finger lifts is a swipe.
            let swipe = cx
                .local
                .gesture
                .update(finger.map(|t| Point::new(t.x, t.y)), now);

            // The blue button: a short press taps whatever is next, holding
            // it for a second goes back to the beginning.
            let pressed = cx.local.button.is_pressed();
            let mut button_tap = false;
            let mut button_restart = false;
            match (pressed, *cx.local.pressed_since) {
                (true, None) => {
                    *cx.local.pressed_since = Some(now);
                    *cx.local.long_press_done = false;
                }
                (true, Some(down)) => {
                    if !*cx.local.long_press_done && now.wrapping_sub(down) >= LONG_PRESS_MS {
                        *cx.local.long_press_done = true;
                        button_restart = true;
                    }
                }
                (false, Some(_)) => {
                    *cx.local.pressed_since = None;
                    button_tap = !*cx.local.long_press_done;
                }
                (false, None) => {}
            }

            let rng = &mut *cx.local.rng;
            let (fx, tap_turned, progress) = cx.shared.model.lock(|model| {
                let before = model.scene;
                let mut fx = update(model, &story, rng, Event::Tick { now_ms: now });
                let mut all = fx;
                let mut tap = |model: &mut Model, at: Point, all: &mut Effects| {
                    let f = update(model, &story, rng, Event::Touch(TouchEvent::Down(at)));
                    merge(all, &f);
                    f
                };
                let mut tap_turned = false;
                if let (true, Some(t)) = (finger_down, finger) {
                    defmt::info!("touch: raw {} -> screen ({}, {})", t.raw, t.x, t.y);
                    fx = tap(model, Point::new(t.x, t.y), &mut all);
                    tap_turned = fx.scene_changed;
                }
                if button_tap {
                    let at = next_tap(model, &story);
                    defmt::info!("button -> tap at ({}, {})", at.x, at.y);
                    fx = tap(model, at, &mut all);
                }
                if button_restart {
                    defmt::info!("button held -> back to the beginning");
                    fx = update(model, &story, rng, Event::Restart);
                    merge(&mut all, &fx);
                }
                if let (true, Some(t)) = (finger_held, finger) {
                    let at = Point::new(t.x, t.y);
                    fx = update(model, &story, rng, Event::Touch(TouchEvent::Move(at)));
                    merge(&mut all, &fx);
                }
                if finger_up {
                    let at = Point::new(0, 0);
                    fx = update(model, &story, rng, Event::Touch(TouchEvent::Up(at)));
                    merge(&mut all, &fx);
                }
                if let Some(s) = swipe {
                    match s {
                        Swipe::Left => defmt::info!("swipe: left"),
                        Swipe::Right => defmt::info!("swipe: right"),
                    }
                    fx = update(model, &story, rng, Event::Swipe(s));
                    merge(&mut all, &fx);
                }
                if model.scene != before || all.reveal.is_some() {
                    log_scene(model);
                }
                all.leds = fx.leds;
                (all, tap_turned, model.progress())
            });
            // Keep what changed, in case the power goes.
            if progress != cx.local.saved.progress {
                save_progress(cx.local.save, cx.local.saved, progress, cx.local.watchdog);
            }
            // The tap that started this touch already turned the page, so
            // the same touch must not turn it again as a swipe.
            if tap_turned {
                cx.local.gesture.cancel();
            }

            if fx.chapter_complete {
                defmt::info!("chapter complete");
            }
            cx.shared.cues.lock(|c| c.record(&fx, now));
            let (green, red) = fx.leds;
            LED_LEVELS.store(u16::from(green) << 8 | u16::from(red), Ordering::Relaxed);
        }
    }

    /// Dims the LEDs: 10 000 times a second, switches each one on or off so
    /// that over time it is on for its share (see `app::leds::Dither`).
    #[task(binds = TIM2, priority = 3, local = [leds, led_timer, dither: Dither = Dither::new()])]
    fn led_pwm(cx: led_pwm::Context) {
        cx.local.led_timer.ack();
        let [green, red] = LED_LEVELS.load(Ordering::Relaxed).to_be_bytes();
        let (g, r) = cx.local.dither.step((green, red));
        cx.local.leds.set(g, r);
    }

    /// Folds the one-shot cues of `f` into `all`.
    fn merge(all: &mut Effects, f: &Effects) {
        all.redraw |= f.redraw;
        all.scene_changed |= f.scene_changed;
        all.chapter_complete |= f.chapter_complete;
        all.dodged |= f.dodged;
        all.ripple = f.ripple.or(all.ripple);
        all.reveal = f.reveal.or(all.reveal);
        all.nudge = f.nudge.or(all.nudge);
    }

    /// Draws the screen on every second LCD refresh (about 30 times a second),
    /// one band of rows at a time. Each band is copied into the framebuffer
    /// only once the LCD controller has read past it, so no refresh shows half
    /// of one frame and half of the next (see `app::pacing`).
    #[task(priority = 1, shared = [model, cues], local = [
        display,
        painter,
        cordic,
        aes,
        stats,
        max_wait,
        band: [u16; BAND_LEN] = [0; BAND_LEN],
        first: bool = true,
        in_step: bool = true,
        secret_opened: bool = false,
        power: app::state::Power = app::state::Power::Awake,
    ])]
    async fn render(mut cx: render::Context) {
        loop {
            if *cx.local.in_step && !wait_for_top_blank().await {
                *cx.local.in_step = false;
                defmt::warn!("lcd: line counter not moving, drawing without waiting for it");
            }
            let in_step = *cx.local.in_step;
            let frame_start = Mono::now();
            let started = now_ms();
            let model = cx.shared.model.lock(|m| *m);
            let cues = cx.shared.cues.lock(|c| *c);

            // Dimmed or asleep while waiting untouched; a touch wakes it.
            let power = model.power(started);
            if power != *cx.local.power {
                *cx.local.power = power;
                let display = &mut *cx.local.display;
                match power {
                    Power::Awake => {
                        display.set_fade_color(ui::theme::BACKGROUND_RGB);
                        display.set_backlight(true);
                        defmt::info!("screen: awake");
                    }
                    Power::Dim => {
                        display.set_fade_color([0, 0, 0]);
                        display.set_backlight(true);
                        defmt::info!("screen: dimmed");
                    }
                    Power::Asleep => {
                        display.set_backlight(false);
                        defmt::info!("screen: asleep");
                    }
                }
            }
            if power == Power::Asleep {
                // Nothing to draw. Still count a frame, so the watchdog knows
                // this task is fine, and look again in a moment.
                FRAMES.fetch_add(1, Ordering::Relaxed);
                Mono::delay(SLEEP_POLL_MS.millis()).await;
                continue;
            }

            let painter = &mut *cx.local.painter;
            // She said yes: open the secret message with the AES engine.
            if model.scene == app::Scene::Celebration && !*cx.local.secret_opened {
                *cx.local.secret_opened = true;
                painter.set_secret(open_secret(cx.local.aes));
            }
            painter.prepare_with(&model, &cues, started, &mut HwTrig(cx.local.cordic));

            let band = &mut cx.local.band[..];
            let max_wait = *cx.local.max_wait;
            let mut racer = Racer::new(SCAN, Display::scan_line());
            let (mut draw, mut copy) = (0u32, 0u32);
            for (i, rows) in cx
                .local
                .display
                .framebuffer()
                .chunks_mut(BAND_LEN)
                .enumerate()
            {
                let t0 = DWT::cycle_count();
                let mut target = Band::new(&mut band[..rows.len()], i * BAND_ROWS);
                let _ = painter.draw(&mut target);
                let t1 = DWT::cycle_count();
                // Wait until the LCD controller has read past this band.
                let bottom = u16::try_from((i + 1) * BAND_ROWS).unwrap_or(u16::MAX);
                while in_step && !racer.may_copy(Display::scan_line(), bottom) {
                    if DWT::cycle_count().wrapping_sub(t1) > max_wait {
                        break;
                    }
                }
                let t2 = DWT::cycle_count();
                rows.copy_from_slice(target.pixels());
                let t3 = DWT::cycle_count();
                draw = draw.wrapping_add(t1.wrapping_sub(t0));
                copy = copy.wrapping_add(t3.wrapping_sub(t2));
            }

            // The layer fade and the sticker, taken by the LCD controller at
            // the next blank.
            let display = &mut *cx.local.display;
            display.set_opacity(match power {
                app::state::Power::Dim => app::screen::DIM_OPACITY,
                _ => app::screen::opacity(&model, started),
            });
            display.place_sticker(app::screen::sticker_spot(&model, started));

            FRAMES.fetch_add(1, Ordering::Relaxed);
            if let Some(r) = cx.local.stats.record(draw, copy, now_ms()) {
                defmt::info!(
                    "frames: {} fps, draw {} us (max {}), copy {} us, busy {}%",
                    r.frames,
                    r.draw_avg_us,
                    r.draw_max_us,
                    r.copy_avg_us,
                    r.busy_pct
                );
            }

            if *cx.local.first {
                *cx.local.first = false;
                defmt::info!(
                    "lcd: first frame drawn in {} ms",
                    now_ms().wrapping_sub(started)
                );
            }

            let lead = if in_step { FRAME_LEAD_MS } else { FRAME_MS };
            Mono::delay_until(frame_start + lead.millis()).await;
        }
    }
}
