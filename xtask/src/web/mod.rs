pub mod server;

use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Splits "GET /path HTTP/1.1" into the method and the target.
pub fn parse_request_line(line: &str) -> (&str, &str) {
    let mut parts = line.split_whitespace();
    (parts.next().unwrap_or(""), parts.next().unwrap_or("/"))
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Maps a request target to a file below `root`. `None` for anything that
/// would leave the folder or is not a file.
pub fn resolve(root: &Path, target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next()?;
    let decoded = percent_decode(path)?;
    if decoded.contains('\0') || decoded.contains('\\') {
        return None;
    }
    let rel = decoded.trim_start_matches('/');
    let rel_path = Path::new(rel);
    if rel_path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut full = root.join(rel_path);
    if rel.is_empty() || rel.ends_with('/') {
        full = full.join("index.html");
    }
    let full = full.canonicalize().ok()?;
    let base = root.canonicalize().ok()?;
    (full.starts_with(&base) && full.is_file()).then_some(full)
}

/// The content type for a file, by its extension.
pub fn mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ttf") => "font/ttf",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Opens `url` in the default browser, or tells the user to do it.
pub fn open_browser(url: &str) {
    let result = if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", "", url]).status()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    };
    if result.is_err() {
        println!("Open {url} in your browser.");
    }
}

#[cfg(test)]
mod tests {
    use super::{mime, parse_request_line, resolve};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xtask-web-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn mime_types_cover_the_simulator_files() {
        assert_eq!(mime(Path::new("sim.wasm")), "application/wasm");
        assert_eq!(mime(Path::new("index.html")), "text/html; charset=utf-8");
        assert_eq!(mime(Path::new("a/door.svg")), "image/svg+xml");
        assert_eq!(mime(Path::new("f.TTF")), "font/ttf");
    }

    #[test]
    fn unknown_files_are_plain_binary() {
        assert_eq!(mime(Path::new("x.bin")), "application/octet-stream");
        assert_eq!(mime(Path::new("noextension")), "application/octet-stream");
    }

    #[test]
    fn request_line_is_split() {
        assert_eq!(
            parse_request_line("GET /a.js HTTP/1.1\r\n"),
            ("GET", "/a.js")
        );
        assert_eq!(parse_request_line(""), ("", "/"));
    }

    #[test]
    fn resolve_serves_files_and_the_index() {
        let root = temp_dir("serve");
        fs::write(root.join("index.html"), "home").unwrap();
        fs::create_dir_all(root.join("icons")).unwrap();
        fs::write(root.join("icons/door.svg"), "svg").unwrap();
        assert!(resolve(&root, "/").unwrap().ends_with("index.html"));
        assert!(resolve(&root, "/icons/door.svg?v=2")
            .unwrap()
            .ends_with("door.svg"));
        assert!(
            resolve(&root, "/icons/").is_none(),
            "no index in that folder"
        );
        assert!(resolve(&root, "/missing.js").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_refuses_to_leave_the_folder() {
        let outer = temp_dir("outer");
        let root = outer.join("web");
        fs::create_dir_all(&root).unwrap();
        fs::write(outer.join("secret.txt"), "no").unwrap();
        for bad in [
            "/../secret.txt",
            "/%2e%2e/secret.txt",
            "/..%2fsecret.txt",
            "/a\\..\\secret.txt",
            "/x%00y",
        ] {
            assert!(resolve(&root, bad).is_none(), "{bad} must be refused");
        }
        let _ = fs::remove_dir_all(&outer);
    }

    #[test]
    fn bad_percent_escapes_are_refused() {
        let root = temp_dir("escape");
        fs::write(root.join("index.html"), "x").unwrap();
        assert!(resolve(&root, "/%zz").is_none());
        assert!(resolve(&root, "/%4").is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
