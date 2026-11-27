#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use defmt_rtt as _;
use panic_probe as _;

use app::state::{NEXT_BUTTON, RESTART_BUTTON};
use app::story::Content;
use app::{Model, Point, Rect, Scene, Story};
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1_000);

/// How often the logic gets a tick and the touch panel is read, in milliseconds.
const TICK_MS: u32 = 16;
/// Holding the blue button this long goes back to the beginning.
const LONG_PRESS_MS: u32 = 1_000;
/// How often the screen is drawn again, in milliseconds (about 30 frames a second).
const FRAME_MS: u32 = 33;
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

#[rtic::app(device = bsp::hal::pac, dispatchers = [SPI1, SPI2])]
mod firmware {
    use app::frame_stats::FrameStats;
    use app::leds::Dither;
    use app::liveness::Liveness;
    use app::screen::{Cues, Painter};
    use app::story::Content;
    use app::{update, Effects, Event, Model, Point, TouchEvent, XorShift32};
    use bsp::{Board, Button, Display, LedTimer, Leds, Touch, Watchdog};
    use cortex_m::peripheral::DWT;
    use rtic_monotonics::systick::prelude::*;
    use ui::frame::Band;

    use super::{
        log_scene, next_tap, now_ms, Mono, Ordering, BAND_LEN, BAND_ROWS, FRAMES, FRAME_MS,
        LED_LEVELS, LONG_PRESS_MS, TICK_MS,
    };

    #[shared]
    struct Shared {
        model: Model,
        cues: Cues,
    }

    #[local]
    struct Local {
        leds: Leds,
        led_timer: LedTimer,
        button: Button,
        touch: Touch,
        display: Display,
        stats: FrameStats,
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
            display,
            touch,
            watchdog,
            reset_by_watchdog,
            core_clock_hz,
            seed,
        } = Board::init(cx.device);
        if reset_by_watchdog {
            defmt::warn!("started again after a hang");
        }
        defmt::info!("core clock {} Hz", core_clock_hz);

        Mono::start(core.SYST, core_clock_hz);

        // The cycle counter times each frame for the once-a-second log line.
        core.DCB.enable_trace();
        DWT::unlock();
        core.DWT.enable_cycle_counter();

        let model = Model::new(0);
        log_scene(&model);

        tick::spawn().ok();
        render::spawn().ok();

        (
            Shared {
                model,
                cues: Cues::new(),
            },
            Local {
                leds,
                led_timer,
                button,
                touch,
                display,
                stats: FrameStats::new(core_clock_hz, 0),
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
    #[task(priority = 2, shared = [model, cues], local = [button, touch, watchdog, liveness, rng, pressed_since, long_press_done, was_touching])]
    async fn tick(mut cx: tick::Context) {
        let story = Content;
        let mut next = Mono::now();
        loop {
            next += TICK_MS.millis();
            Mono::delay_until(next).await;
            let now = now_ms();

            // Feed the watchdog only while the screen keeps drawing too.
            if cx.local.liveness.alive(FRAMES.load(Ordering::Relaxed), now) {
                cx.local.watchdog.feed();
            }

            // A finger that just touched the glass is a tap.
            let finger = cx.local.touch.read();
            let finger_down = finger.is_some() && !*cx.local.was_touching;
            *cx.local.was_touching = finger.is_some();

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
            let fx = cx.shared.model.lock(|model| {
                let before = model.scene;
                let mut fx = update(model, &story, rng, Event::Tick { now_ms: now });
                let mut all = fx;
                let mut tap = |model: &mut Model, at: Point, all: &mut Effects| {
                    let f = update(model, &story, rng, Event::Touch(TouchEvent::Down(at)));
                    merge(all, &f);
                    f
                };
                if let (true, Some(t)) = (finger_down, finger) {
                    defmt::info!("touch: raw {} -> screen ({}, {})", t.raw, t.x, t.y);
                    fx = tap(model, Point::new(t.x, t.y), &mut all);
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
                if model.scene != before || all.reveal.is_some() {
                    log_scene(model);
                }
                all.leds = fx.leds;
                all
            });

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

    /// Draws the screen about 30 times a second, one band of rows at a time.
    #[task(priority = 1, shared = [model, cues], local = [
        display,
        stats,
        painter: Painter = Painter::new(),
        band: [u16; BAND_LEN] = [0; BAND_LEN],
        first: bool = true,
    ])]
    async fn render(mut cx: render::Context) {
        let mut next = Mono::now();
        loop {
            let started = now_ms();
            let model = cx.shared.model.lock(|m| *m);
            let cues = cx.shared.cues.lock(|c| *c);
            let painter = &mut *cx.local.painter;
            painter.prepare(&model, &cues, started);

            let band = &mut cx.local.band[..];
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
                rows.copy_from_slice(target.pixels());
                let t2 = DWT::cycle_count();
                draw = draw.wrapping_add(t1.wrapping_sub(t0));
                copy = copy.wrapping_add(t2.wrapping_sub(t1));
            }

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

            next += FRAME_MS.millis();
            let now = Mono::now();
            if next < now {
                next = now;
            }
            Mono::delay_until(next).await;
        }
    }
}
