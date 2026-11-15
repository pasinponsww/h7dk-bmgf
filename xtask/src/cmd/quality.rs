//! The cargo housekeeping commands: `test`, `check` and `fmt`.

use std::process::Command;

use crate::tools::paths::ARM_TARGET;
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

/// `cargo proj common test`: every test that needs no board.
pub fn test() -> Result<()> {
    exec(step_pc(&["test", "--workspace"], &[]))
}

/// `cargo proj common fmt`: formats the whole workspace.
pub fn fmt() -> Result<()> {
    exec(step(&["fmt", "--all"]))
}

/// `cargo proj common check`: format, lint, tests and the firmware build, stopping at the first failure.
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
