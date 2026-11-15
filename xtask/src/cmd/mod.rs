//! The commands, grouped by what they work on, and the router that picks one:
//!
//! - `common`: status, setup (`health`), test, check, fmt (`quality`)
//! - `mbedded`: build, flash, run, attach, reset, detect (`board`)

pub mod board;
pub mod health;
pub mod quality;

use crate::cli::{Cli, Cmd, CommonCmd, MbeddedCmd};
use crate::tools::process::Result;

/// Runs the command the user asked for.
pub fn dispatch(cli: Cli) -> Result<()> {
    // The app crate's build script reads this when it builds, so every cargo command that
    // follows uses the chosen story.
    std::env::set_var("H7_STORY", &cli.story);
    match cli.cmd {
        Cmd::Common { cmd } => match cmd {
            CommonCmd::Status => health::status(),
            CommonCmd::Setup { yes } => health::setup(yes),
            CommonCmd::Test => quality::test(),
            CommonCmd::Check => quality::check(),
            CommonCmd::Fmt => quality::fmt(),
        },
        Cmd::Mbedded { cmd } => match cmd {
            MbeddedCmd::Build { dev, flash: true } => board::flash(!dev, false),
            MbeddedCmd::Build { dev, flash: false } => board::build(!dev),
            MbeddedCmd::Flash { dev, no_build } => board::flash(!dev, no_build),
            MbeddedCmd::Run { dev } => board::run(!dev),
            MbeddedCmd::Attach { dev } => board::attach(!dev),
            MbeddedCmd::Reset => board::reset(),
            MbeddedCmd::Detect => board::detect(),
        },
    }
}
