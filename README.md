# h7dk-bmgf

A proposal story for the STM32H735G-DK touchscreen, written in embedded Rust.

## Setup on Windows

Install Rust and the Visual Studio **Desktop development with C++** workload. Then open a new terminal and run:

```powershell
cargo proj common setup
```

This installs any missing tools and Rust targets.

## Commands

Use `cargo proj` to run project commands. (`cargo xtask` works too.)

```powershell
cargo proj common status   # Check tools, board, and build files
cargo proj common setup    # Install missing tools
cargo proj common test     # Run logic tests on your PC
cargo proj common check    # Run formatting, lint, tests, and the firmware build
cargo proj common fmt      # Format the code

cargo proj mbedded build   # Build the firmware and print flash and RAM use
cargo proj mbedded build --flash   # Build, flash and reset in one step
cargo proj mbedded flash   # Build and flash the board
cargo proj mbedded run     # Flash the board and show its log
cargo proj mbedded attach  # Show the board log
cargo proj mbedded reset   # Reset the board
cargo proj mbedded detect  # Show the connected board
```

Add `--help` to any command for more information.

Short names: under `common`, `s` status, `t` test and `ci` check. Under `mbedded`, `b` or `compile` build, `f` flash and `r` run.

Use `--story <folder>` to choose a story:

```powershell
cargo proj mbedded build --story personal
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
- `xtask/` — The `cargo proj` command-line helper
- `crates/` — Reusable libraries for geometry, random numbers, and board support
- `story/` — Story files
- `assets/` — Icons and fonts

Reusable code goes in `crates/`. Code that is specific to this project stays in the top-level folders. The story logic does not depend on the board.

## Licenses

The icons are from Phosphor Icons and use the MIT license. The license is in `assets/icons/phosphor/LICENSE`.

The Press Start 2P and VT323 fonts use the SIL Open Font License. Their license files are next to the fonts in `assets/fonts/pixel/`.
