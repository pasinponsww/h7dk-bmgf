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

#[rtic::app(device = bsp::hal::pac, dispatchers = [SPI1, SPI2])]
mod firmware {
    use app::story::Content;
    use app::{update, Event, Model, Point, TouchEvent, XorShift32};
    use bsp::{Board, Button, Display, Leds, Touch};
    use rtic_monotonics::systick::prelude::*;

    use super::{log_scene, next_tap, Mono, LONG_PRESS_MS, TICK_MS};

    #[shared]
    struct Shared {
        model: Model,
        leds: Leds,
    }

    #[local]
    struct Local {
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
        } = Board::init(cx.device);
        defmt::info!("core clock {} Hz", core_clock_hz);

        Mono::start(core.SYST, core_clock_hz);

        let model = Model::new(0);
        log_scene(&model);

        tick::spawn().ok();
        render::spawn().ok();

        (
            Shared { model, leds },
            Local {
                button,
                touch,
                display,
                rng: XorShift32::new(0xA5A5_1234),
                pressed_since: None,
                long_press_done: false,
                was_touching: false,
            },
        )
    }

    /// Runs the logic: the clock, the touch panel and the blue button.
    #[task(shared = [model, leds], local = [button, touch, rng, pressed_since, long_press_done, was_touching])]
    async fn tick(mut cx: tick::Context) {
        let story = Content;
        let mut next = Mono::now();
        loop {
            next += TICK_MS.millis();
            Mono::delay_until(next).await;
            let now_ms = Mono::now().duration_since_epoch().to_millis();

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
                    *cx.local.pressed_since = Some(now_ms);
                    *cx.local.long_press_done = false;
                }
                (true, Some(down)) => {
                    if !*cx.local.long_press_done && now_ms.wrapping_sub(down) >= LONG_PRESS_MS {
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
            let (fx, redraw) = cx.shared.model.lock(|model| {
                let before = model.scene;
                let mut fx = update(model, &story, rng, Event::Tick { now_ms });
                let mut redraw = fx.redraw;
                if let (true, Some(t)) = (finger_down, finger) {
                    defmt::info!("touch: raw {} -> screen ({}, {})", t.raw, t.x, t.y);
                    let at = Point::new(t.x, t.y);
                    fx = update(model, &story, rng, Event::Touch(TouchEvent::Down(at)));
                    redraw |= fx.redraw;
                }
                if button_tap {
                    let at = next_tap(model, &story);
                    defmt::info!("button -> tap at ({}, {})", at.x, at.y);
                    fx = update(model, &story, rng, Event::Touch(TouchEvent::Down(at)));
                    redraw |= fx.redraw;
                }
                if button_restart {
                    defmt::info!("button held -> back to the beginning");
                    fx = update(model, &story, rng, Event::Restart);
                    redraw |= fx.redraw;
                }
                if model.scene != before || fx.reveal.is_some() {
                    log_scene(model);
                }
                (fx, redraw)
            });

            if fx.chapter_complete {
                defmt::info!("chapter complete");
            }
            cx.shared.leds.lock(|leds| leds.set(fx.leds.0, fx.leds.1));
            if redraw {
                let _ = render::spawn();
            }
        }
    }

    #[task(local = [display])]
    async fn render(cx: render::Context) {
        cx.local
            .display
            .framebuffer()
            .fill(ui::theme::to_raw(ui::theme::BACKGROUND));
        defmt::info!("lcd: painted the background color");
    }
}
