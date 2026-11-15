//! The commands, grouped by what they work on, and the router that picks one:
//!
//! - `board`: build, size, flash, run, attach, reset
//! - `quality`: test, check, fmt, clean
//! - `health`: status, setup

pub mod board;
pub mod health;
pub mod quality;

use crate::cli::{Cli, Cmd};
use crate::tools::process::Result;

/// Runs the command the user asked for.
pub fn dispatch(cli: Cli) -> Result<()> {
    // The app crate's build script reads this when it builds, so every cargo command that
    // follows uses the chosen story.
    std::env::set_var("H7_STORY", &cli.story);
    match cli.cmd {
        Cmd::Status => health::status(),
        Cmd::Setup { yes } => health::setup(yes),
        Cmd::Test => quality::test(),
        Cmd::Check => quality::check(),
        Cmd::Fmt => quality::fmt(),
        Cmd::Build { dev } => board::build(!dev),
        Cmd::Size { dev } => board::size(!dev),
        Cmd::Flash { dev, no_build } => board::flash(!dev, no_build),
        Cmd::Run { dev } => board::run(!dev),
        Cmd::Attach { dev } => board::attach(!dev),
        Cmd::Reset => board::reset(),
        Cmd::Clean => quality::clean(),
    }
}
