//! The cargo housekeeping commands: `test`, `check`, `fmt` and `clean`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tools::paths::{root, ARM_TARGET};
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

/// `cargo proj check`: format, lint, tests and the firmware build, stopping at the first failure.
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

/// The helper's own program files inside `target/`: the running one, and its copies in `deps/`.
/// Empty when the helper runs from somewhere else, such as `cargo install`.
fn own_binaries(target: &Path) -> Vec<PathBuf> {
    let (Ok(me), Ok(target)) = (env::current_exe(), target.canonicalize()) else {
        return Vec::new();
    };
    let Ok(me) = me.canonicalize() else {
        return Vec::new();
    };
    if !me.starts_with(&target) {
        return Vec::new();
    }
    let mut files = vec![me.clone()];
    let prefix = format!("{}-", me.file_stem().unwrap_or_default().to_string_lossy());
    if let Some(deps) = me.parent().map(|p| p.join("deps")) {
        for entry in fs::read_dir(deps).into_iter().flatten().flatten() {
            let path = entry.path();
            let same_kind = path.extension() == me.extension();
            let name = entry.file_name().to_string_lossy().into_owned();
            if same_kind && name.starts_with(&prefix) {
                files.push(path);
            }
        }
    }
    files
}

/// Moves files into `dir`. Windows refuses to delete a program that is running,
/// but it lets the program be renamed, which frees its place for `cargo clean`.
fn park(files: &[PathBuf], dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    for (i, file) in files.iter().enumerate() {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        fs::rename(file, dir.join(format!("{i}-{name}")))
            .map_err(|e| format!("could not move {} aside: {e}", file.display()))?;
    }
    Ok(())
}

/// `cargo proj clean`: deletes all build output, then builds this helper again so the
/// next `cargo proj` starts at once.
pub fn clean() -> Result<()> {
    let parked = env::temp_dir().join(format!("h7dk-proj-parked-{}", std::process::id()));
    park(&own_binaries(&root().join("target")), &parked)?;
    let cleaned = exec(step(&["clean"]));
    let rebuilt = exec(step(&["build", "-p", "xtask"]));
    let _ = fs::remove_dir_all(&parked);
    cleaned.and(rebuilt)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{own_binaries, park};

    #[test]
    fn parking_moves_files_out_of_the_way() {
        let tmp = std::env::temp_dir().join(format!("xtask-park-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let file = tmp.join("proj.exe");
        fs::write(&file, "x").unwrap();
        park(std::slice::from_ref(&file), &tmp.join("parked")).unwrap();
        assert!(!file.exists());
        assert!(tmp.join("parked/0-proj.exe").is_file());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn nothing_is_parked_when_the_helper_runs_outside_the_folder() {
        let elsewhere = std::env::temp_dir().join("xtask-not-the-target");
        fs::create_dir_all(&elsewhere).unwrap();
        assert!(own_binaries(&elsewhere).is_empty());
    }
}
