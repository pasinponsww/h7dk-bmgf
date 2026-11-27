#![no_std]
#![no_main]

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

fn now_ms() -> u32 {
    Mono::now().duration_since_epoch().to_millis()
}

#[rtic::app(device = bsp::hal::pac, dispatchers = [SPI1, SPI2])]
mod firmware {
    use app::screen::{Cues, Painter};
    use app::story::Content;
    use app::{update, Effects, Event, Model, Point, TouchEvent, XorShift32};
    use bsp::{Board, Button, Display, Leds, Touch};
    use rtic_monotonics::systick::prelude::*;
    use ui::frame::Band;

    use super::{
        log_scene, next_tap, now_ms, Mono, BAND_LEN, BAND_ROWS, FRAME_MS, LONG_PRESS_MS, TICK_MS,
    };

    #[shared]
    struct Shared {
        model: Model,
        cues: Cues,
    }

    #[local]
    struct Local {
        leds: Leds,
        button: Button,
        touch: Touch,
        display: Display,
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
        // Instruction cache only: the data cache would hide new pixels from
        // the LCD controller and log messages from the debugger.
        core.SCB.enable_icache();

        let Board {
            leds,
            button,
            display,
            touch,
            core_clock_hz,
            seed,
        } = Board::init(cx.device);
        defmt::info!("core clock {} Hz", core_clock_hz);

        Mono::start(core.SYST, core_clock_hz);

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
                button,
                touch,
                display,
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
    #[task(priority = 2, shared = [model, cues], local = [leds, button, touch, rng, pressed_since, long_press_done, was_touching])]
    async fn tick(mut cx: tick::Context) {
        let story = Content;
        let mut next = Mono::now();
        loop {
            next += TICK_MS.millis();
            Mono::delay_until(next).await;
            let now = now_ms();

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
            cx.local.leds.set(fx.leds.0, fx.leds.1);
        }
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
            for (i, rows) in cx
                .local
                .display
                .framebuffer()
                .chunks_mut(BAND_LEN)
                .enumerate()
            {
                let mut target = Band::new(&mut band[..rows.len()], i * BAND_ROWS);
                let _ = painter.draw(&mut target);
                rows.copy_from_slice(target.pixels());
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
