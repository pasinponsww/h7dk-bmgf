//! `cargo proj story`: check and list the story folders.

use std::fs;
use std::process::Command;

use crate::tools::paths::{root, story_dir, story_names};
use crate::tools::process::{cargo, run as exec, Result};

/// Counts `(chapters, objects)` in the text of a `story.toml`.
pub fn summarize(text: &str) -> (usize, usize) {
    let count = |tag: &str| text.lines().filter(|l| l.trim() == tag).count();
    (count("[[chapter]]"), count("[[chapter.object]]"))
}

/// True if git ignores this story folder, meaning it is kept private.
fn is_private(name: &str) -> bool {
    Command::new("git")
        .args(["check-ignore", "-q", &format!("story/{name}/story.toml")])
        .current_dir(root())
        .status()
        .is_ok_and(|s| s.success())
}

/// `cargo proj story list`
pub fn list() -> Result<()> {
    let names = story_names();
    if names.is_empty() {
        return Err("no story folders found in story/".to_string());
    }
    let active = std::env::var("H7_STORY").unwrap_or_else(|_| "default".to_string());
    println!("Story folders (the one marked * is used by --story):\n");
    for name in names {
        let text =
            fs::read_to_string(story_dir().join(&name).join("story.toml")).unwrap_or_default();
        let (chapters, objects) = summarize(&text);
        println!(
            "  {} {name:<12} {chapters:>2} chapters, {objects:>2} objects   {}",
            if name == active { "*" } else { " " },
            if is_private(&name) {
                "private (ignored by git)"
            } else {
                "in git"
            }
        );
    }
    Ok(())
}

/// `cargo proj story check`
pub fn check(name: &str) -> Result<()> {
    let names = story_names();
    if !names.iter().any(|n| n == name) {
        return Err(format!(
            "no story called \"{name}\". Folders with a story.toml: {}",
            names.join(", ")
        ));
    }
    std::env::set_var("H7_STORY", name);
    println!("Checking the story \"{name}\" (the build stops with a message if a line, icon or count is wrong)");
    let mut c = cargo();
    c.args(["test", "-p", "app", "--lib", "common::story::tests"]);
    exec(c)?;
    println!("\nThe story \"{name}\" is valid.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::summarize;
    use crate::tools::paths::{story_dir, story_names};

    #[test]
    fn chapters_and_objects_are_counted() {
        let text = "[[chapter]]\ntitle = \"A\"\n\n[[chapter.object]]\nicon = \"door\"\n\n[[chapter.object]]\nicon = \"star\"\n\n[[chapter]]\ntitle = \"B\"\n";
        assert_eq!(summarize(text), (2, 2));
        assert_eq!(summarize(""), (0, 0));
    }

    #[test]
    fn every_story_folder_has_chapters_with_objects() {
        for name in story_names() {
            let text = std::fs::read_to_string(story_dir().join(&name).join("story.toml")).unwrap();
            let (chapters, objects) = summarize(&text);
            assert!(chapters >= 1, "{name} has no chapters");
            assert!(objects >= chapters * 2, "{name} has too few objects");
        }
    }
}
