# h7dk-bmgf

A touch-screen proposal box for the STM32H735G-DK, written in embedded Rust.
The user taps through story chapters (tappable objects on each page), then gets
the question. The "No" button runs away five times, then gives in.

The story itself is data, kept in `story/`. The repo ships a short placeholder
story; a real one lives in a private folder (see [Story](#story)).

## Status

| Part | State |
|---|---|
| Logic (`src/`) | Done, 26 tests, organised by event source (`src/task/`) |
| Story loader (`build.rs`, `src/common/story.rs`) | Done, 7 tests that hold for any story. Reads `story/<name>/story.toml` at build time |
| Generic libraries (`crates/geometry`, `crates/rng`) | Done, 12 tests |
| Color theme (`ui`) | Done, 2 tests. Drawing code not started |
| Browser simulator (`sim`) | Done. Runs the real logic in WebAssembly, 4 tests. Look is a CSS stand-in until `ui` draws pixels |
| Helper CLI (`xtask`) | Done. `cargo proj status/build/flash/attach/sim/story ...`, 32 tests, includes a small web server |
| Firmware (`src/main.rs`) | Built on RTIC: a 16 ms `tick` task runs the logic and the LEDs, a `render` task paints the LCD the theme background, and the blue button acts as a tap, so the whole story can be walked without drawing anything. Compile-checked only |
| Board support (`crates/bsp`, one file: `board.rs`) | `Board`, the two LEDs, the user button and the LCD (LTDC, framebuffer, backlight) are written. Compile-checked only, not run on a real board yet |
| Drawing and touch | Nothing is drawn yet except a solid background color. The touch controller pins are found (see `crates/bsp/README.md`) but its driver is not written |
| On-board test | Not done yet: the ST-LINK was not detected by `probe-rs list` |

## Setup (Windows)

```powershell
winget install Rustlang.Rustup
rustup default stable
rustup target add thumbv7em-none-eabihf wasm32-unknown-unknown
rustup component add llvm-tools rust-src clippy rustfmt
cargo install probe-rs-tools flip-link --locked
```

The Visual Studio "Desktop development with C++" workload is also needed.
Open a new terminal after installing so `cargo` is on the PATH. After that
`cargo proj setup` installs whatever is still missing, so you do not need to
type the lines above by hand.

## Commands

One helper does everything: `cargo proj <command>` (`cargo xtask <command>` is
the same thing). Start with `cargo proj status`, which checks the tools and the
board and says what to do next.

```powershell
cargo proj status          # tools, board, build outputs, and what to do next
cargo proj setup           # install anything missing (targets, probe-rs)

cargo proj test            # all logic tests on the PC, no board needed
cargo proj check           # every gate: format, lint, tests, firmware + simulator builds
cargo proj fmt             # format the code

cargo proj sim             # build the browser simulator, serve it, open it
cargo proj plan            # open the local planning page (docs/plan)

cargo proj story list      # the story folders, and which are private
cargo proj story check     # check a story: counts, line lengths, icons, a full playthrough

cargo proj build           # build the firmware (--dev for a debug build)
cargo proj size            # flash and RAM used, as a share of the chip
cargo proj flash           # build, flash and reset the board
cargo proj run             # build, flash and show the board's log
cargo proj attach          # show the board's log without flashing
cargo proj reset           # reset the board
cargo proj clean           # delete build output
```

Add `--help` to any command, for example `cargo proj sim --help`. Everything
works without the board except `flash`, `run`, `attach` and `reset`.

`--story <folder>` works on every command and picks the story to build in, for
example `cargo proj sim --story personal`. The default is `default`.

To type `proj` without `cargo`, run `cargo install --path xtask` once.

Plain `cargo test` also works and needs no flags. The firmware is the one part
that only builds for the chip, behind the `firmware` feature, so a normal PC
build never touches it. `cargo proj build` turns the feature on for you.

## Story

The story is data, not code. Each folder in `story/` holds one `story.toml`.

| Folder | In git? | What it is |
|---|---|---|
| `story/default/` | yes | A short placeholder story, so the project runs out of the box |
| `story/personal/` | no | A real story. It is its own private git repo and the main repo ignores it |

The format and the rules (two to four objects per chapter, line length, valid
icon names) are in `story/README.md`. A bad story stops the build with a message
that says what to fix.

## Layout

```
Cargo.toml      the root package "app" (library + firmware) and the workspace
build.rs        loads the chosen story folder, and links the chip's memory map
memory.x        the chip's flash and RAM layout
src/
  lib.rs        the library: hardware-free logic, takes Events, returns Effects
  main.rs       the firmware, built only with the `firmware` feature
  common/       what every task shares: state.rs, event.rs, story.rs (trait + loaded story)
  task/         one file per thing that happens: touch.rs, tick.rs, leds.rs
ui/             colors (RGB565) now; scenes and drawing later (embedded-graphics)
sim/            the browser simulator: the same logic compiled to WebAssembly,
                plus web/ (the page; the wasm, icons and fonts are copied in by `cargo proj sim`)
xtask/          the `cargo proj` helper
  src/cmd/      the commands, grouped: board, quality, preview, health, story
  src/tools/    shared helpers: process (run programs, rustup), paths (and copying), console, firmware (probe, size report)
  src/web/      the small web server behind `sim` and `plan`
crates/         generic libraries, reusable in any project (nothing here knows about this one)
  geometry/     Point, Rect, hit testing, and "farthest free spot" placement
  rng/          Rng trait and a deterministic XorShift32
  bsp/          board support for the STM32H735G-DK in one file: Board, LEDs, button, LCD (chip-only, see its README)
story/          the story folders (default in git, personal private)
assets/
  icons/        Phosphor Icons (MIT), duotone. See assets/icons/README.md
  fonts/        pixel fonts (Press Start 2P, VT323), SIL OFL. See assets/fonts/README.md
docs/plan/      local-only planning page (git-ignored)
```

Rules the layout follows:

- If it could be copied into another project unchanged, it goes in `crates/`.
  If it knows about the proposal box, it is a top-level folder.
- Tests sit next to the code they check, in a `#[cfg(test)] mod tests` at the
  bottom of the file. Helpers shared by the logic tests are in
  `src/common/testing.rs`.
- `Cargo.toml` files have no comments.

Dependencies point inward: the firmware and `sim` use the library, the library
uses the generic `crates/`, and none of the logic knows about the chip.

## How the logic works

`app::update(&mut model, &story, &mut rng, event) -> Effects`

- `Event::Tick { now_ms }` every 10 to 16 ms, `Event::Touch(TouchEvent::Down(p))`.
- Intro: any touch opens chapter 1.
- Story: tapping an object reveals its line and flashes both LEDs for 350 ms.
  "Next" unlocks when every object is found, or after 9 s idle. After 3 s idle
  an unfound object gets a pulse hint.
- Question: a touch within 40 px of "No" makes it jump (never onto YES). After
  5 jumps it stops and becomes "Okay, yes". YES grows 6% per jump, up to 150%.
- Celebration returns to the intro after 60 s idle.
- LEDs: a pure function of scene and time (`app::task::leds::sample`).

## Licenses

Icons are from Phosphor Icons, MIT license. The license text is in
`assets/icons/phosphor/LICENSE`.

The pixel fonts (Press Start 2P and VT323) are under the SIL Open Font License;
the license texts are next to the font files in `assets/fonts/pixel/`.
