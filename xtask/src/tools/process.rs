//! Running programs, reading what they print, and asking what rustup has installed.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::paths::root;

/// Errors are plain messages shown to the user.
pub type Result<T> = std::result::Result<T, String>;

/// `~/.cargo/bin`, where rustup and `cargo install` put tools.
fn cargo_bin() -> Option<PathBuf> {
    let home = env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| {
        env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(|h| PathBuf::from(h).join(".cargo"))
    })?;
    Some(home.join("bin"))
}

/// A command for an installed tool. Looks in `~/.cargo/bin` first, so it
/// works even when that folder is not on the PATH of this terminal.
pub fn tool(name: &str) -> Command {
    if let Some(bin) = cargo_bin() {
        for file in [format!("{name}.exe"), name.to_string()] {
            let p = bin.join(&file);
            if p.is_file() {
                let mut c = Command::new(p);
                c.current_dir(root());
                return c;
            }
        }
    }
    let mut c = Command::new(name);
    c.current_dir(root());
    c
}

/// A `cargo` command that starts in the workspace root.
pub fn cargo() -> Command {
    let mut c = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    c.current_dir(root());
    c
}

/// The command as the user would type it, for display.
pub fn describe(cmd: &Command) -> String {
    let prog = Path::new(cmd.get_program())
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| {
            let s = a.to_string_lossy();
            // Show long absolute paths by their file name only.
            if Path::new(s.as_ref()).is_absolute() {
                Path::new(s.as_ref())
                    .file_name()
                    .map_or_else(|| s.to_string(), |f| f.to_string_lossy().into_owned())
            } else {
                s.into_owned()
            }
        })
        .collect();
    format!("{prog} {}", args.join(" ")).trim().to_string()
}

/// Runs a command with its output shown live. Fails if it exits with an error.
pub fn run(mut cmd: Command) -> Result<()> {
    println!("==> {}", describe(&cmd));
    let status = cmd
        .status()
        .map_err(|e| format!("could not start `{}`: {e}", describe(&cmd)))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{}` failed ({status})", describe(&cmd)))
    }
}

/// Runs a command quietly and returns its combined output.
pub fn capture(mut cmd: Command) -> Result<String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text.trim().to_string())
    } else {
        Err(text.trim().to_string())
    }
}

/// Rust targets rustup has installed.
pub fn installed_targets() -> Vec<String> {
    let mut c = tool("rustup");
    c.args(["target", "list", "--installed"]);
    capture(c)
        .map(|t| t.lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

/// The rustup components that are installed, one per line.
pub fn installed_components() -> String {
    let mut c = tool("rustup");
    c.args(["component", "list", "--installed"]);
    capture(c).unwrap_or_default()
}

/// Fails with a fix-it message if a Rust target is not installed.
pub fn require_target(target: &str) -> Result<()> {
    if installed_targets().iter().any(|t| t == target) {
        Ok(())
    } else {
        Err(format!(
            "the Rust target {target} is not installed. Run: cargo proj common setup"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::describe;
    use std::process::Command;

    #[test]
    fn describe_shows_program_and_args() {
        let mut c = Command::new("cargo");
        c.args(["build", "-p", "app"]);
        assert_eq!(describe(&c), "cargo build -p app");
    }

    #[test]
    fn describe_shortens_absolute_paths_to_the_file_name() {
        let mut c = Command::new("probe-rs");
        c.arg("download");
        c.arg(std::env::current_dir().unwrap().join("firmware.elf"));
        assert_eq!(describe(&c), "probe-rs download firmware.elf");
    }
}
