//! The commands that build the firmware and talk to the board:
//! `detect`, `build`, `flash`, `run`, `attach` and `reset`.

use crate::tools::firmware::{measure, print_sizes, require_probe};
use crate::tools::paths::{elf_path, ensure_elf, ARM_TARGET, CHIP};
use crate::tools::process::{cargo, require_target, run as exec, tool, Result};

/// `cargo proj mbedded build`: builds the firmware and prints how much of the chip it uses.
pub fn build(release: bool) -> Result<()> {
    require_target(ARM_TARGET)?;
    let mut c = cargo();
    c.args([
        "build",
        "-p",
        "app",
        "--bin",
        "h7dk-fw",
        "--features",
        "firmware",
        "--target",
        ARM_TARGET,
    ]);
    if release {
        c.arg("--release");
    }
    exec(c)?;
    match measure(release) {
        Ok(s) => print_sizes(&s),
        Err(_) => println!("(add the size report with: rustup component add llvm-tools)"),
    }
    Ok(())
}

/// `cargo proj mbedded detect`: prints the connected board, or says how to connect one.
pub fn detect() -> Result<()> {
    println!("board: {}", require_probe()?);
    Ok(())
}

/// `cargo proj mbedded flash`: flashes the firmware (building it first unless
/// `no_build`) and resets the board.
pub fn flash(release: bool, no_build: bool) -> Result<()> {
    println!("board: {}", require_probe()?);
    if !no_build {
        build(release)?;
    }
    let elf = ensure_elf(release)?;
    let mut c = tool("probe-rs");
    c.args(["download", "--chip", CHIP]).arg(&elf);
    exec(c)?;
    reset()?;
    println!("\nFlashed and reset. Watch the log with: cargo proj mbedded attach");
    Ok(())
}

/// `cargo proj mbedded run`: builds, flashes and stays attached until Ctrl+C.
pub fn run(release: bool) -> Result<()> {
    println!("board: {}", require_probe()?);
    build(release)?;
    let mut c = tool("probe-rs");
    c.args(["run", "--chip", CHIP]).arg(elf_path(release));
    exec(c)
}

/// `cargo proj mbedded attach`: shows the board's log without flashing.
pub fn attach(release: bool) -> Result<()> {
    println!("board: {}", require_probe()?);
    let elf = ensure_elf(release)?;
    let mut c = tool("probe-rs");
    c.args(["attach", "--chip", CHIP]).arg(&elf);
    exec(c)
}

/// `cargo proj mbedded reset`: resets the board.
pub fn reset() -> Result<()> {
    require_probe()?;
    let mut c = tool("probe-rs");
    c.args(["reset", "--chip", CHIP]);
    exec(c)
}
