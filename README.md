# h7dk-bmgf

A proposal story for the STM32H735G-DK touchscreen, written in embedded Rust.

## Setup on Windows

Install Rust and the Visual Studio **Desktop development with C++** workload. Then open a new terminal and run:

```powershell
cargo proj setup
```

This installs any missing tools and Rust targets.

## Commands

Use `cargo proj` to run project commands. (`cargo xtask` works too.)

```powershell
cargo proj status          # Check tools, board, and build files
cargo proj setup           # Install missing tools
cargo proj test            # Run logic tests on your PC
cargo proj check           # Run formatting, lint, tests, and builds
cargo proj fmt             # Format the code

cargo proj sim             # Build and open the browser simulator
cargo proj plan            # Open the planning page

cargo proj story list      # List available stories
cargo proj story check     # Check a story for errors

cargo proj build           # Build the firmware
cargo proj size            # Show flash and RAM usage
cargo proj flash           # Build and flash the board
cargo proj run             # Flash the board and show its log
cargo proj attach          # Show the board log
cargo proj reset           # Reset the board
cargo proj clean           # Delete build files
```

Add `--help` to any command for more information.

Use `--story <folder>` to choose a story:

```powershell
cargo proj sim --story personal
```

The default story is `default`. To run `proj` without typing `cargo`, install it once:

```powershell
cargo install --path xtask
```

You can also run `cargo test` directly. It runs the PC tests without building the firmware.

## Stories

Stories are kept in `story/`. Each story has a `story.toml` file.

| Folder | In git? | Description |
|---|---|---|
| `story/default/` | Yes | A short sample story |
| `story/personal/` | No | A private story stored in its own git repository |

See `story/README.md` for the story format and rules. The build reports errors if a story is invalid.

## Project folders

- `src/` — Story logic and firmware
- `src/task/` — Code for touch, timing, and LEDs
- `ui/` — Colors and, later, screen drawing
- `sim/` — Browser simulator
- `xtask/` — The `cargo proj` command-line helper
- `crates/` — Reusable libraries for geometry, random numbers, and board support
- `story/` — Story files
- `assets/` — Icons and fonts
- `docs/plan/` — Local planning page; not included in git

Reusable code goes in `crates/`. Code that is specific to this project stays in the top-level folders. The story logic does not depend on the board.

## Licenses

The icons are from Phosphor Icons and use the MIT license. The license is in `assets/icons/phosphor/LICENSE`.

The Press Start 2P and VT323 fonts use the SIL Open Font License. Their license files are next to the fonts in `assets/fonts/pixel/`.
