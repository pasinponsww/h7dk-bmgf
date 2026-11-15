//! The command line.

use clap::{Parser, Subcommand};

/// Project helper for the H7 proposal box.
#[derive(Parser)]
#[command(
    name = "proj",
    version,
    arg_required_else_help = true,
    after_help = "Start with `cargo proj status`. Everything works without the board except flash, run, attach and reset."
)]
pub struct Cli {
    /// Which story to build in: a folder inside story/ (see story/README.md)
    #[arg(long, global = true, default_value = "default")]
    pub story: String,

    #[command(subcommand)]
    pub cmd: Cmd,
}

/// Everything `cargo proj` can do.
#[derive(Subcommand)]
pub enum Cmd {
    /// Check the tools, the board and the build outputs, and say what to do next
    Status,
    /// Install anything that is missing (Rust targets, probe-rs)
    Setup {
        /// Do not ask before installing
        #[arg(long, short)]
        yes: bool,
    },
    /// Run all the logic tests on the PC (no board needed)
    Test,
    /// Run every quality gate: format, lint, tests and the firmware build
    Check,
    /// Format the code
    Fmt,
    /// Build the firmware for the board
    Build {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
    },
    /// Show how much flash and RAM the firmware uses
    Size {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
    },
    /// Build and flash the firmware, then reset the board
    Flash {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
        /// Flash the existing build without rebuilding
        #[arg(long)]
        no_build: bool,
    },
    /// Build, flash and stay attached to show the board's log (Ctrl+C to stop)
    Run {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
    },
    /// Show the board's log without flashing (Ctrl+C to stop)
    Attach {
        /// Use the debug build to decode the log
        #[arg(long)]
        dev: bool,
    },
    /// Reset the board
    Reset,
    /// Delete all build output, then rebuild this helper so `cargo proj` keeps working
    Clean,
}
