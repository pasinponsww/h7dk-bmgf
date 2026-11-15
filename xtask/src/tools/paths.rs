//! Where things are, and the fixed names of this project.

use std::path::{Path, PathBuf};

use super::process::Result;

/// The chip on the STM32H735G-DK, as probe-rs names it.
pub const CHIP: &str = "STM32H735IGKx";
/// Rust target of the chip.
pub const ARM_TARGET: &str = "thumbv7em-none-eabihf";
/// Name of the firmware binary.
pub const FIRMWARE_BIN: &str = "h7dk-fw";

/// The workspace root (the folder above `xtask`).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside the workspace root")
        .to_path_buf()
}

/// Where cargo puts the firmware ELF.
pub fn elf_path(release: bool) -> PathBuf {
    root()
        .join("target")
        .join(ARM_TARGET)
        .join(if release { "release" } else { "debug" })
        .join(FIRMWARE_BIN)
}

/// The firmware ELF, or an error telling the user to build it first.
pub fn ensure_elf(release: bool) -> Result<PathBuf> {
    let elf = elf_path(release);
    if elf.is_file() {
        Ok(elf)
    } else {
        Err("no firmware build found. Run: cargo proj mbedded build".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{elf_path, root, FIRMWARE_BIN};

    #[test]
    fn root_is_the_workspace() {
        assert!(root().join("Cargo.toml").is_file());
        assert!(root().join("xtask").is_dir());
    }

    #[test]
    fn elf_paths_differ_by_profile_and_end_with_the_binary_name() {
        assert_ne!(elf_path(true), elf_path(false));
        assert!(elf_path(true).ends_with(FIRMWARE_BIN));
        assert!(elf_path(true).to_string_lossy().contains("release"));
        assert!(elf_path(false).to_string_lossy().contains("debug"));
    }

    #[test]
    fn the_firmware_package_has_the_name_the_helper_expects() {
        let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
        assert!(manifest.contains(&format!("name = \"{FIRMWARE_BIN}\"")));
    }
}
