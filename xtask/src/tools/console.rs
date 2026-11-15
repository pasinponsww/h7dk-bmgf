//! Small things for talking to the user.

use std::io::{self, Write};
use std::path::Path;
use std::time::SystemTime;

/// Prints a section title.
pub fn header(title: &str) {
    println!("\n{title}");
}

/// "5 min ago" for a file's last change.
pub fn age(path: &Path) -> Option<String> {
    let modified = path.metadata().ok()?.modified().ok()?;
    let secs = SystemTime::now().duration_since(modified).ok()?.as_secs();
    Some(match secs {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    })
}

/// Bytes as "12.3 KB".
pub fn kb(bytes: u64) -> String {
    format!("{:.1} KB", bytes as f64 / 1024.0)
}

/// Asks a yes/no question. Returns true without asking when `assume_yes`.
pub fn confirm(question: &str, assume_yes: bool) -> bool {
    if assume_yes {
        return true;
    }
    print!("{question} [y/N] ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).is_ok() && matches!(answer.trim(), "y" | "Y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::{age, confirm, kb};
    use std::fs;
    use std::path::Path;

    #[test]
    fn kb_formats_one_decimal() {
        assert_eq!(kb(0), "0.0 KB");
        assert_eq!(kb(1024), "1.0 KB");
        assert_eq!(kb(1536), "1.5 KB");
    }

    #[test]
    fn a_file_changed_just_now_says_so() {
        let path = std::env::temp_dir().join(format!("xtask-age-{}", std::process::id()));
        fs::write(&path, "x").unwrap();
        assert_eq!(age(&path).as_deref(), Some("just now"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_missing_file_has_no_age() {
        assert_eq!(age(Path::new("definitely-not-here.txt")), None);
    }

    #[test]
    fn confirm_with_yes_does_not_ask() {
        assert!(confirm("Install?", true));
    }
}
