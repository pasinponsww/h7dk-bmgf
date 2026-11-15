#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use app::state::NEXT_BUTTON;
use app::story::Content;
use app::{Model, Point, Rect, Scene, Story};
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1_000);

/// How often the logic gets a tick, in milliseconds.
const TICK_MS: u32 = 16;

fn center(r: Rect) -> Point {
    Point::new(r.x + r.w / 2, r.y + r.h / 2)
}

/// The tap that moves the story one step forward. The button stands in for the
/// touchscreen, so it taps whatever is next: the intro, the next object, the
/// Next button, or YES.
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
        Scene::Celebration => Point::new(0, 0),
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
    use app::{update, Event, Model, TouchEvent, XorShift32};
    use bsp::{Board, Button, Display, Leds};
    use rtic_monotonics::systick::prelude::*;

    use super::{log_scene, next_tap, Mono, TICK_MS};

    #[shared]
    struct Shared {
        model: Model,
        leds: Leds,
    }

    #[local]
    struct Local {
        button: Button,
        display: Display,
        rng: XorShift32,
        was_pressed: bool,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("h7dk-fw boot");
        let Board {
            leds,
            button,
            display,
            core_clock_hz,
        } = Board::init(cx.device);
        defmt::info!("core clock {} Hz", core_clock_hz);

        Mono::start(cx.core.SYST, core_clock_hz);

        let model = Model::new(0);
        log_scene(&model);

        tick::spawn().ok();
        render::spawn().ok();

        (
            Shared { model, leds },
            Local {
                button,
                display,

                rng: XorShift32::new(0xA5A5_1234),
                was_pressed: false,
            },
        )
    }

    #[task(shared = [model, leds], local = [button, rng, was_pressed])]
    async fn tick(mut cx: tick::Context) {
        let story = Content;
        let mut next = Mono::now();
        loop {
            next += TICK_MS.millis();
            Mono::delay_until(next).await;
            let now_ms = Mono::now().duration_since_epoch().to_millis();

            let pressed = cx.local.button.is_pressed();
            let tapped = pressed && !*cx.local.was_pressed;
            *cx.local.was_pressed = pressed;

            let fx = cx.shared.model.lock(|model| {
                let mut fx = update(model, &story, cx.local.rng, Event::Tick { now_ms });
                if tapped {
                    let tap = next_tap(model, &story);
                    defmt::info!("button -> tap at ({}, {})", tap.x, tap.y);
                    fx = update(
                        model,
                        &story,
                        cx.local.rng,
                        Event::Touch(TouchEvent::Down(tap)),
                    );
                }
                if fx.scene_changed || fx.reveal.is_some() {
                    log_scene(model);
                }
                fx
            });

            if fx.chapter_complete {
                defmt::info!("chapter complete");
            }
            cx.shared.leds.lock(|leds| leds.set(fx.leds.0, fx.leds.1));
            if fx.redraw {
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
