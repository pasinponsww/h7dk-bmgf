//! The cargo housekeeping commands: `test`, `check`, `fmt` and `clean`.

use std::process::Command;

use crate::tools::paths::{root, web_dir, ARM_TARGET, WASM_TARGET};
use crate::tools::process::{cargo, run as exec, Result};

/// The crates that only build for the chip. They cannot run on the PC.
const CHIP_ONLY: [&str; 2] = ["--exclude", "bsp"];

fn step(args: &[&str]) -> Command {
    let mut c = cargo();
    c.args(args);
    c
}

fn step_pc(args: &[&str], tail: &[&str]) -> Command {
    let mut c = cargo();
    c.args(args).args(CHIP_ONLY).args(tail);
    c
}

/// `cargo proj test`: every test that needs no board.
pub fn test() -> Result<()> {
    exec(step_pc(&["test", "--workspace"], &[]))
}

/// `cargo proj fmt`: formats the whole workspace.
pub fn fmt() -> Result<()> {
    exec(step(&["fmt", "--all"]))
}

/// `cargo proj check`: format, lint, tests and both builds, stopping at the first failure.
pub fn check() -> Result<()> {
    let steps: Vec<(&str, Command)> = vec![
        ("format", step(&["fmt", "--all", "--", "--check"])),
        (
            "lint (PC crates)",
            step_pc(
                &["clippy", "--workspace"],
                &["--all-targets", "--", "-D", "warnings"],
            ),
        ),
        (
            "lint (firmware)",
            step(&[
                "clippy",
                "-p",
                "bsp",
                "-p",
                "app",
                "--features",
                "firmware",
                "--target",
                ARM_TARGET,
                "--",
                "-D",
                "warnings",
            ]),
        ),
        ("tests", step_pc(&["test", "--workspace"], &[])),
        (
            "firmware build",
            step(&[
                "build",
                "-p",
                "app",
                "--bin",
                "h7dk-fw",
                "--features",
                "firmware",
                "--release",
                "--target",
                ARM_TARGET,
            ]),
        ),
        (
            "simulator build",
            step(&["build", "-p", "sim", "--release", "--target", WASM_TARGET]),
        ),
    ];

    let mut passed: Vec<&str> = Vec::new();
    for (name, cmd) in steps {
        println!("\n--- {name} ---");
        if let Err(e) = exec(cmd) {
            println!("\nSummary:");
            for p in &passed {
                println!("  [ ok ] {p}");
            }
            println!("  [ !! ] {name}");
            return Err(e);
        }
        passed.push(name);
    }
    println!("\nSummary:");
    for p in &passed {
        println!("  [ ok ] {p}");
    }
    println!("\nAll checks passed.");
    Ok(())
}

/// `cargo proj clean`: runs `cargo clean` and removes the files `sim` copied next to the page.
pub fn clean() -> Result<()> {
    exec(step(&["clean"]))?;
    let web = web_dir();
    for generated in ["sim.wasm", "icons", "fonts"] {
        let p = web.join(generated);
        let removed = if p.is_dir() {
            std::fs::remove_dir_all(&p)
        } else if p.is_file() {
            std::fs::remove_file(&p)
        } else {
            continue;
        };
        removed.map_err(|e| format!("could not remove {}: {e}", p.display()))?;
        println!("removed {}", p.strip_prefix(root()).unwrap_or(&p).display());
    }
    Ok(())
}
