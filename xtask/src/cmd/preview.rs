//! The commands that show something in the browser: `sim` and `plan`.

use std::fs;

use crate::tools::console::kb;
use crate::tools::paths::{copy_files, plan_dir, root, web_dir, WASM_TARGET};
use crate::tools::process::{cargo, require_target, run as exec, Result};
use crate::web::server::serve;

/// `cargo proj sim`: builds the browser simulator (unless `build` is false) and serves it.
pub fn sim(port: u16, open: bool, build: bool) -> Result<()> {
    if build {
        build_wasm()?;
    }
    let web = web_dir();
    if !web.join("sim.wasm").is_file() {
        return Err("no simulator build yet. Run: cargo proj sim".to_string());
    }
    serve(&web, port, open, "simulator")
}

/// `cargo proj plan`: serves `docs/plan`, which is local-only and not in git.
pub fn plan(port: u16, open: bool) -> Result<()> {
    let dir = plan_dir();
    if !dir.join("index.html").is_file() {
        return Err(
            "docs/plan/index.html is missing (that folder is local-only and not in git)"
                .to_string(),
        );
    }
    serve(&dir, port, open, "plan page")
}

fn build_wasm() -> Result<()> {
    require_target(WASM_TARGET)?;
    let mut c = cargo();
    c.args(["build", "-p", "sim", "--release", "--target", WASM_TARGET])
        .env("CARGO_PROFILE_RELEASE_DEBUG", "0")
        .env("CARGO_PROFILE_RELEASE_STRIP", "symbols");
    exec(c)?;

    let root = root();
    let web = web_dir();
    fs::create_dir_all(&web).map_err(|e| e.to_string())?;
    let wasm = root
        .join("target")
        .join(WASM_TARGET)
        .join("release/sim.wasm");
    fs::copy(&wasm, web.join("sim.wasm"))
        .map_err(|e| format!("could not copy {}: {e}", wasm.display()))?;
    let icons = copy_files(
        &root.join("assets/icons/phosphor/duotone"),
        &web.join("icons"),
        "svg",
    )?;
    let fonts = copy_files(&root.join("assets/fonts/pixel"), &web.join("fonts"), "ttf")?;
    let size = fs::metadata(web.join("sim.wasm"))
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "\nBuilt sim.wasm ({}), copied {icons} icons and {fonts} fonts.",
        kb(size)
    );
    Ok(())
}
