//! The command line.

use clap::{Parser, Subcommand};

/// Project helper for the H7 proposal box.
#[derive(Parser)]
#[command(
    name = "proj",
    version,
    arg_required_else_help = true,
    after_help = "Start with `cargo proj common status`. Everything under `common` works without the board."
)]
pub struct Cli {
    /// Which story to build in: a folder inside story/ (see story/README.md)
    #[arg(long, global = true, default_value = "default")]
    pub story: String,

    #[command(subcommand)]
    pub cmd: Cmd,
}

/// The two groups of commands.
#[derive(Subcommand)]
pub enum Cmd {
    /// Everyday project commands: status, setup, test, check, fmt
    Common {
        #[command(subcommand)]
        cmd: CommonCmd,
    },
    /// Firmware and board commands: build, flash, run, attach, reset, detect
    Mbedded {
        #[command(subcommand)]
        cmd: MbeddedCmd,
    },
}

/// `cargo proj common ...`: nothing here needs the board.
#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
pub enum CommonCmd {
    /// Check the tools, the board and the build outputs, and say what to do next
    #[command(visible_alias = "s")]
    Status,
    /// Install anything that is missing (Rust targets, probe-rs)
    Setup {
        /// Do not ask before installing
        #[arg(long, short)]
        yes: bool,
    },
    /// Run all the logic tests on the PC (no board needed)
    #[command(visible_alias = "t")]
    Test,
    /// Run every quality gate: format, lint, tests and the firmware build
    #[command(visible_alias = "ci")]
    Check,
    /// Format the code
    Fmt,
}

/// `cargo proj mbedded ...`: the firmware and the board.
#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
pub enum MbeddedCmd {
    /// Build the firmware and print its flash and RAM use (`--flash` also flashes it)
    #[command(visible_aliases = ["compile", "b"])]
    Build {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
        /// Flash the board after building
        #[arg(long, short)]
        flash: bool,
    },
    /// Build and flash the firmware, then reset the board
    #[command(visible_alias = "f")]
    Flash {
        /// Debug build instead of release
        #[arg(long)]
        dev: bool,
        /// Flash the existing build without rebuilding
        #[arg(long)]
        no_build: bool,
    },
    /// Build, flash and stay attached to show the board's log (Ctrl+C to stop)
    #[command(visible_alias = "r")]
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
    /// Show the connected board
    Detect,
    /// Turn a folder of photos into an album and write it to the board's
    /// 64 MB flash (shown as a slideshow on the celebration screen)
    Photos {
        /// Folder with .jpg, .jpeg or .png files (the first 64, by name)
        folder: std::path::PathBuf,
        /// Only make target/album.bin; do not write it to the board
        #[arg(long)]
        no_flash: bool,
        /// STM32CubeProgrammer external loader for the board's flash, if it
        /// is not found next to the programmer
        #[arg(long)]
        loader: Option<std::path::PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Cmd, CommonCmd, MbeddedCmd};

    fn parse(args: &[&str]) -> Cmd {
        Cli::try_parse_from(std::iter::once("proj").chain(args.iter().copied()))
            .unwrap()
            .cmd
    }

    fn common(args: &[&str]) -> CommonCmd {
        let all: Vec<&str> = std::iter::once("common")
            .chain(args.iter().copied())
            .collect();
        match parse(&all) {
            Cmd::Common { cmd } => cmd,
            Cmd::Mbedded { .. } => panic!("expected a common command"),
        }
    }

    fn mbedded(args: &[&str]) -> MbeddedCmd {
        let all: Vec<&str> = std::iter::once("mbedded")
            .chain(args.iter().copied())
            .collect();
        match parse(&all) {
            Cmd::Mbedded { cmd } => cmd,
            Cmd::Common { .. } => panic!("expected an mbedded command"),
        }
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn the_everyday_commands_live_under_common() {
        assert!(matches!(common(&["status"]), CommonCmd::Status));
        assert!(matches!(common(&["setup"]), CommonCmd::Setup { .. }));
        assert!(matches!(common(&["test"]), CommonCmd::Test));
        assert!(matches!(common(&["check"]), CommonCmd::Check));
        assert!(matches!(common(&["fmt"]), CommonCmd::Fmt));
    }

    #[test]
    fn the_board_commands_live_under_mbedded() {
        assert!(matches!(mbedded(&["build"]), MbeddedCmd::Build { .. }));
        assert!(matches!(mbedded(&["flash"]), MbeddedCmd::Flash { .. }));
        assert!(matches!(mbedded(&["run"]), MbeddedCmd::Run { .. }));
        assert!(matches!(mbedded(&["attach"]), MbeddedCmd::Attach { .. }));
        assert!(matches!(mbedded(&["reset"]), MbeddedCmd::Reset));
        assert!(matches!(mbedded(&["detect"]), MbeddedCmd::Detect));
        assert!(matches!(
            mbedded(&["photos", "pics", "--no-flash"]),
            MbeddedCmd::Photos { no_flash: true, .. }
        ));
    }

    #[test]
    fn the_old_top_level_names_and_removed_commands_are_gone() {
        for old in ["status", "build", "flash", "size", "clean", "sim", "plan"] {
            assert!(
                Cli::try_parse_from(["proj", old]).is_err(),
                "{old} should not be top level"
            );
        }
        assert!(Cli::try_parse_from(["proj", "mbedded", "size"]).is_err());
        assert!(Cli::try_parse_from(["proj", "common", "clean"]).is_err());
    }

    #[test]
    fn short_names_reach_the_same_commands() {
        assert!(matches!(common(&["s"]), CommonCmd::Status));
        assert!(matches!(common(&["t"]), CommonCmd::Test));
        assert!(matches!(common(&["ci"]), CommonCmd::Check));
        assert!(matches!(mbedded(&["f"]), MbeddedCmd::Flash { .. }));
        assert!(matches!(mbedded(&["r"]), MbeddedCmd::Run { .. }));
        assert!(matches!(mbedded(&["b"]), MbeddedCmd::Build { .. }));
    }

    #[test]
    fn compile_is_build_and_can_flash() {
        assert!(matches!(
            mbedded(&["compile"]),
            MbeddedCmd::Build {
                dev: false,
                flash: false
            }
        ));
        assert!(matches!(
            mbedded(&["compile", "--flash", "--dev"]),
            MbeddedCmd::Build {
                dev: true,
                flash: true
            }
        ));
        assert!(matches!(
            mbedded(&["b", "-f"]),
            MbeddedCmd::Build { flash: true, .. }
        ));
    }
}
