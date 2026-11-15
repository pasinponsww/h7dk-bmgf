//! The server loop.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use super::{mime, open_browser, parse_request_line, resolve};
use crate::tools::process::Result;

/// Serves `root` on localhost until the process is stopped.
pub fn serve(root: &Path, port: u16, open: bool, name: &str) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
        format!("cannot use port {port}: {e}\n  Another server may already be using it. Pick another port with --port.")
    })?;
    let url = format!("http://localhost:{port}/");
    println!("\nServing the {name} at {url}\nPress Ctrl+C to stop.");
    if open {
        open_browser(&url);
    }
    run_server(listener, root.to_path_buf());
    Ok(())
}

/// Accepts connections forever, one thread each.
pub fn run_server(listener: TcpListener, root: PathBuf) {
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        thread::spawn(move || {
            let _ = handle(stream, &root);
        });
    }
}

fn handle(mut stream: TcpStream, root: &Path) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
    }
    let (method, target) = parse_request_line(&request_line);
    if method != "GET" && method != "HEAD" {
        return respond(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
            false,
        );
    }
    let head_only = method == "HEAD";
    match resolve(root, target).and_then(|p| fs::read(&p).ok().map(|body| (p, body))) {
        Some((path, body)) => respond(&mut stream, 200, mime(&path), &body, head_only),
        None => respond(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"not found",
            head_only,
        ),
    }
}

fn respond(
    stream: &mut TcpStream,
    code: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> io::Result<()> {
    let reason = match code {
        200 => "OK",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::run_server;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::thread;

    fn fetch(addr: SocketAddr, request: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_all(request.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn the_server_answers_over_a_real_socket() {
        let root = std::env::temp_dir().join(format!("xtask-socket-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("index.html"), "hello sim").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let r = root.clone();
        thread::spawn(move || run_server(listener, r));

        let ok = fetch(addr, "GET / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(ok.starts_with("HTTP/1.1 200 OK"), "{ok}");
        assert!(ok.contains("Content-Type: text/html"));
        assert!(ok.ends_with("hello sim"));

        let head = fetch(addr, "HEAD / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(head.starts_with("HTTP/1.1 200 OK") && !head.contains("hello sim"));

        assert!(fetch(addr, "GET /nope HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(fetch(addr, "GET /../Cargo.toml HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(fetch(addr, "POST / HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 405"));

        let _ = fs::remove_dir_all(&root);
    }
}
