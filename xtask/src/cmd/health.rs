//! The commands that look at the machine: `status` (is everything ready, and
//! what to do next) and `setup` (install what is missing).

use crate::tools::console::{age, confirm, header, kb};
use crate::tools::firmware::{probe, probe_rs_installed};
use crate::tools::paths::{elf_path, root, ARM_TARGET};
use crate::tools::process::{
    capture, cargo, installed_components, installed_targets, run as exec, tool, Result,
};

#[derive(Clone, Copy)]
enum Mark {
    Ok,
    Missing,
    Optional,
}

fn row(mark: Mark, label: &str, detail: &str) {
    let tag = match mark {
        Mark::Ok => "[ ok ]",
        Mark::Missing => "[ !! ]",
        Mark::Optional => "[ -- ]",
    };
    if detail.is_empty() {
        println!("  {tag} {label}");
    } else {
        println!("  {tag} {label:<34} {detail}");
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

fn git(args: &[&str]) -> Option<String> {
    let mut c = std::process::Command::new("git");
    c.args(args).current_dir(root());
    capture(c).ok()
}

/// `cargo proj status`: prints the state of the tools, the board, the build
/// outputs and the project.
pub fn status() -> Result<()> {
    let mut next: Vec<String> = Vec::new();

    header("Tools");
    let mut c = tool("rustc");
    c.arg("--version");
    match capture(c) {
        Ok(v) => row(Mark::Ok, "rustc", &first_line(&v)),
        Err(_) => {
            row(Mark::Missing, "rustc", "not found");
            next.push(
                "Install Rust: winget install Rustlang.Rustup, then open a new terminal.".into(),
            );
        }
    }
    let targets = installed_targets();
    if targets.iter().any(|x| x == ARM_TARGET) {
        row(Mark::Ok, &format!("target {ARM_TARGET}"), "firmware");
    } else {
        row(
            Mark::Missing,
            &format!("target {ARM_TARGET}"),
            "needed for the firmware",
        );
        next.push("Run: cargo proj setup".into());
    }
    let mut c = tool("probe-rs");
    c.arg("--version");
    match capture(c) {
        Ok(v) => row(Mark::Ok, "probe-rs", &first_line(&v)),
        Err(_) => {
            row(Mark::Missing, "probe-rs", "flash and debug need it");
            next.push("Run: cargo proj setup".into());
        }
    }
    if installed_components().contains("llvm-tools") {
        row(Mark::Ok, "llvm-tools", "firmware size report");
    } else {
        row(
            Mark::Optional,
            "llvm-tools",
            "optional, for the size report",
        );
    }

    header("Board");
    match probe() {
        Ok(Some(p)) => row(Mark::Ok, "debug probe", &p),
        Ok(None) => {
            row(Mark::Missing, "debug probe", "no board detected");
            next.push(
                "Plug the ST-LINK USB port into the PC, try another cable, then run: cargo proj status"
                    .into(),
            );
        }
        Err(_) => row(Mark::Optional, "debug probe", "install probe-rs to check"),
    }

    header("Build output");
    let elf = elf_path(true);
    if elf.is_file() {
        let size = elf.metadata().map(|m| kb(m.len())).unwrap_or_default();
        row(
            Mark::Ok,
            "firmware (release)",
            &format!("{size} file, built {}", age(&elf).unwrap_or_default()),
        );
    } else {
        row(Mark::Optional, "firmware (release)", "not built yet");
        next.push("Build it: cargo proj build".into());
    }

    header("Project");
    let active = std::env::var("H7_STORY").unwrap_or_else(|_| "default".to_string());
    row(Mark::Ok, "story", &format!("using \"{active}\""));
    match git(&["status", "--short"]) {
        Some(s) => {
            let changed = s.lines().count();
            let branch = git(&["branch", "--show-current"]).unwrap_or_default();
            let committed = git(&["rev-parse", "HEAD"]).is_some();
            row(
                Mark::Ok,
                "git",
                &format!(
                    "branch {}, {changed} changed file(s){}",
                    if branch.is_empty() { "?" } else { &branch },
                    if committed {
                        ""
                    } else {
                        ", nothing committed yet"
                    }
                ),
            );
        }
        None => row(Mark::Optional, "git", "not available"),
    }

    println!("\nNext:");
    next.dedup();
    if next.is_empty() {
        println!("  Everything is ready. Try: cargo proj flash");
    } else {
        for n in &next {
            println!("  - {n}");
        }
        println!("  - Tests need no board: cargo proj test");
    }
    Ok(())
}

/// One thing `setup` can install.
pub struct Missing {
    /// What it is, in plain words.
    pub what: &'static str,
    /// The program that installs it.
    pub program: &'static str,
    /// The arguments for that program.
    pub args: &'static [&'static str],
}

/// Finds what is missing for the build and the board.
pub fn find_missing() -> Vec<Missing> {
    let mut m = Vec::new();
    let targets = installed_targets();
    if !targets.iter().any(|t| t == ARM_TARGET) {
        m.push(Missing {
            what: "Rust target for the chip",
            program: "rustup",
            args: &["target", "add", ARM_TARGET],
        });
    }
    if !installed_components().contains("llvm-tools") {
        m.push(Missing {
            what: "llvm-tools (firmware size report)",
            program: "rustup",
            args: &["component", "add", "llvm-tools"],
        });
    }
    if !probe_rs_installed() {
        m.push(Missing {
            what: "probe-rs (flash and debug, takes a few minutes)",
            program: "cargo",
            args: &["install", "probe-rs-tools", "--locked"],
        });
    }
    m
}

/// `cargo proj setup`: lists what is missing and installs it after asking
/// (or straight away with `yes`).
pub fn setup(yes: bool) -> Result<()> {
    let missing = find_missing();
    if missing.is_empty() {
        println!("Nothing to install. Everything needed is already there.");
        return Ok(());
    }
    println!("Missing:");
    for m in &missing {
        println!("  - {}  ({} {})", m.what, m.program, m.args.join(" "));
    }
    if !confirm("\nInstall these now?", yes) {
        return Err("cancelled, nothing was installed".to_string());
    }
    for m in &missing {
        let mut c = if m.program == "cargo" {
            cargo()
        } else {
            tool(m.program)
        };
        c.args(m.args);
        exec(c)?;
    }
    println!("\nDone. Check with: cargo proj status");
    Ok(())
}
