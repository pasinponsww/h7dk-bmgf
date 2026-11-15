//! Where things are, the fixed names of this project, and copying files.

use std::fs;
use std::path::{Path, PathBuf};

use super::process::Result;

/// The chip on the STM32H735G-DK, as probe-rs names it.
pub const CHIP: &str = "STM32H735IGKx";
/// Rust target of the chip.
pub const ARM_TARGET: &str = "thumbv7em-none-eabihf";
/// Rust target of the browser simulator.
pub const WASM_TARGET: &str = "wasm32-unknown-unknown";
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
        Err("no firmware build found. Run: cargo proj build".to_string())
    }
}

/// The folder the simulator page is served from.
pub fn web_dir() -> PathBuf {
    root().join("sim/web")
}

/// The folder of the local planning page.
pub fn plan_dir() -> PathBuf {
    root().join("docs/plan")
}

/// The folder that holds the story folders.
pub fn story_dir() -> PathBuf {
    root().join("story")
}

/// Names of the story folders that contain a `story.toml`, sorted.
pub fn story_names() -> Vec<String> {
    let Ok(entries) = fs::read_dir(story_dir()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().join("story.toml").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Copies every file with the given extension from one folder to another,
/// creating the destination. Returns how many were copied.
pub fn copy_files(from: &Path, to: &Path, ext: &str) -> Result<usize> {
    fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))?;
    let mut copied = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == ext) {
            if let Some(name) = path.file_name() {
                fs::copy(&path, to.join(name))
                    .map_err(|e| format!("could not copy {}: {e}", path.display()))?;
                copied += 1;
            }
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{
        copy_files, elf_path, plan_dir, root, story_dir, story_names, web_dir, FIRMWARE_BIN,
    };

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

    #[test]
    fn the_shared_folders_exist() {
        assert!(web_dir().join("index.html").is_file());
        assert!(story_dir().is_dir());
        let _ = plan_dir();
    }

    #[test]
    fn the_default_story_is_always_there() {
        assert!(story_names().iter().any(|n| n == "default"));
    }

    #[test]
    fn story_names_are_sorted() {
        let names = story_names();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn the_asset_folders_have_what_the_simulator_needs() {
        let tmp = std::env::temp_dir().join(format!("xtask-copy-{}", std::process::id()));
        let icons = copy_files(
            &root().join("assets/icons/phosphor/duotone"),
            &tmp.join("icons"),
            "svg",
        )
        .unwrap();
        assert!(icons >= 20, "only {icons} icons found");
        let fonts = copy_files(
            &root().join("assets/fonts/pixel"),
            &tmp.join("fonts"),
            "ttf",
        )
        .unwrap();
        assert_eq!(fonts, 2);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn only_files_with_the_extension_are_copied() {
        let tmp = std::env::temp_dir().join(format!("xtask-ext-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let from = tmp.join("from");
        fs::create_dir_all(&from).unwrap();
        fs::write(from.join("a.svg"), "a").unwrap();
        fs::write(from.join("b.txt"), "b").unwrap();
        let n = copy_files(&from, &tmp.join("to"), "svg").unwrap();
        assert_eq!(n, 1);
        assert!(tmp.join("to/a.svg").is_file());
        assert!(!tmp.join("to/b.txt").exists());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn copying_from_a_missing_folder_is_a_clear_error() {
        let e = copy_files(
            Path::new("definitely-not-here"),
            &std::env::temp_dir().join("x-none"),
            "svg",
        )
        .unwrap_err();
        assert!(e.contains("cannot read"));
    }
}
