//! The commands that build the firmware and talk to the board:
//! `build`, `size`, `flash`, `run`, `attach` and `reset`.

use crate::tools::firmware::{measure, print_sizes, require_probe};
use crate::tools::paths::{elf_path, ensure_elf, ARM_TARGET, CHIP};
use crate::tools::process::{cargo, require_target, run as exec, tool, Result};

/// `cargo proj build`: builds the firmware and prints how much of the chip it uses.
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

/// `cargo proj size`: prints the size of the last firmware build.
pub fn size(release: bool) -> Result<()> {
    let sizes = measure(release).map_err(|e| {
        if e.contains("llvm-size") {
            "the size report needs: rustup component add llvm-tools".to_string()
        } else {
            e
        }
    })?;
    print_sizes(&sizes);
    Ok(())
}

/// `cargo proj flash`: flashes the firmware (building it first unless
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
    println!("\nFlashed and reset. Watch the log with: cargo proj attach");
    Ok(())
}

/// `cargo proj run`: builds, flashes and stays attached until Ctrl+C.
pub fn run(release: bool) -> Result<()> {
    println!("board: {}", require_probe()?);
    build(release)?;
    let mut c = tool("probe-rs");
    c.args(["run", "--chip", CHIP]).arg(elf_path(release));
    exec(c)
}

/// `cargo proj attach`: shows the board's log without flashing.
pub fn attach(release: bool) -> Result<()> {
    println!("board: {}", require_probe()?);
    let elf = ensure_elf(release)?;
    let mut c = tool("probe-rs");
    c.args(["attach", "--chip", CHIP]).arg(&elf);
    exec(c)
}

/// `cargo proj reset`: resets the board.
pub fn reset() -> Result<()> {
    require_probe()?;
    let mut c = tool("probe-rs");
    c.args(["reset", "--chip", CHIP]);
    exec(c)
}
