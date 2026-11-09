//! Bring-up firmware: the real proposal-box logic on the real LEDs, with the
//! LCD painted the theme's background color. No drawing or touch yet.

#![no_std]
#![no_main]

use app::state::NEXT_BUTTON;
use app::story::Content;
use app::{update, Effects, Event, Model, Point, Rect, Scene, Story, TouchEvent, XorShift32};
use bsp::Board;
use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;

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

#[entry]
fn main() -> ! {
    defmt::info!("h7dk-fw boot");
    let mut board = Board::take().expect("board peripherals were already taken");
    defmt::info!("core clock {} Hz", board.core_clock_hz);

    board
        .display
        .framebuffer()
        .fill(ui::theme::to_raw(ui::theme::BACKGROUND));
    defmt::info!("lcd: painted the background color");

    let story = Content;
    let mut model = Model::new(0);
    let mut rng = XorShift32::new(0xA5A5_1234);
    let cycles_per_tick = board.core_clock_hz / 1000 * TICK_MS;

    let mut now_ms = 0u32;
    let mut was_pressed = false;
    log_scene(&model);

    loop {
        now_ms = now_ms.wrapping_add(TICK_MS);
        let mut fx: Effects = update(&mut model, &story, &mut rng, Event::Tick { now_ms });

        let pressed = board.button.is_pressed();
        if pressed && !was_pressed {
            let tap = next_tap(&model, &story);
            defmt::info!("button -> tap at ({}, {})", tap.x, tap.y);
            fx = update(
                &mut model,
                &story,
                &mut rng,
                Event::Touch(TouchEvent::Down(tap)),
            );
        }
        was_pressed = pressed;

        if fx.scene_changed || fx.reveal.is_some() {
            log_scene(&model);
        }
        if fx.chapter_complete {
            defmt::info!("chapter complete");
        }

        board.leds.set(fx.leds.0, fx.leds.1);
        cortex_m::asm::delay(cycles_per_tick);
    }
}
