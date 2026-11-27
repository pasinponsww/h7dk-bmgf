//! `cargo proj mbedded photos <folder>`: turns a folder of photos into a photo
//! album and writes it to the board's 64 MB external flash.
//!
//! The album format is the one in the app's `src/common/album.rs`: a 64-byte
//! "H7ALBUM1" header, then 128 x 128 RGB565 photos, low byte first. Each
//! photo is cut to a centered square, shrunk and Floyd-Steinberg dithered,
//! exactly as `build.rs` does for photos baked into the firmware.
//!
//! probe-rs cannot write the external flash (it has no flash algorithm for
//! it), so the album is written by ST's STM32CubeProgrammer with the board's
//! external loader (`MX25LM51245G_STM32H735G-DK.stldr`, which comes with it).

use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::RgbImage;

use crate::tools::console::kb;
use crate::tools::paths::root;
use crate::tools::process::{run as exec, Result};

/// The first bytes of an album (see the app's `album.rs`).
const MAGIC: &[u8; 8] = b"H7ALBUM1";
/// Bytes before the first photo.
const HEADER_BYTES: usize = 64;
/// Width and height of every photo.
const SIDE: u32 = 128;
/// The most photos the board loads.
const MAX_PHOTOS: usize = 64;
/// Where the external flash appears in the chip's memory map.
const FLASH_ADDRESS: &str = "0x90000000";

/// Floyd-Steinberg dithering to RGB565, the rounding error of each pixel
/// pushed onto its neighbors so gradients stay smooth.
pub fn dither_rgb565(img: &RgbImage) -> Vec<u16> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut work: Vec<[f32; 3]> = img
        .pixels()
        .map(|p| [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])])
        .collect();
    let levels = [31.0f32, 63.0, 31.0];
    let mut out = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut q = [0u16; 3];
            let mut err = [0f32; 3];
            for c in 0..3 {
                let v = work[i][c].clamp(0.0, 255.0);
                let n = (v * levels[c] / 255.0).round();
                q[c] = n as u16;
                err[c] = v - n * 255.0 / levels[c];
            }
            out[i] = q[0] << 11 | q[1] << 5 | q[2];
            let mut push = |dx: isize, dy: usize, share: f32| {
                let nx = x as isize + dx;
                if nx < 0 || nx >= w as isize || y + dy >= h {
                    return;
                }
                let j = (y + dy) * w + nx as usize;
                for c in 0..3 {
                    work[j][c] += err[c] * share;
                }
            };
            push(1, 0, 7.0 / 16.0);
            push(-1, 1, 3.0 / 16.0);
            push(0, 1, 5.0 / 16.0);
            push(1, 1, 1.0 / 16.0);
        }
    }
    out
}

/// `img` cut to a centered square and shrunk to `SIDE` x `SIDE`.
pub fn square(img: &RgbImage) -> RgbImage {
    let side = img.width().min(img.height());
    let x = (img.width() - side) / 2;
    let y = (img.height() - side) / 2;
    let cut = imageops::crop_imm(img, x, y, side, side).to_image();
    imageops::resize(&cut, SIDE, SIDE, FilterType::Lanczos3)
}

/// The album for `photos`: the header, then each photo's pixels.
pub fn album(photos: &[RgbImage]) -> Vec<u8> {
    let mut out = vec![0u8; HEADER_BYTES];
    out[..8].copy_from_slice(MAGIC);
    let count = u32::try_from(photos.len()).unwrap_or(u32::MAX);
    out[8..12].copy_from_slice(&count.to_le_bytes());
    out[12..16].copy_from_slice(&SIDE.to_le_bytes());
    for p in photos {
        out.extend(
            dither_rgb565(&square(p))
                .iter()
                .flat_map(|v| v.to_le_bytes()),
        );
    }
    out
}

/// The photo files in `folder`, sorted by name.
fn photo_files(folder: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(folder)
        .map_err(|e| format!("cannot read the folder {}: {e}", folder.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Where STM32CubeProgrammer's command-line tool usually is.
fn programmer() -> Option<PathBuf> {
    let mut places: Vec<PathBuf> = vec![
        PathBuf::from(r"C:\Program Files\STMicroelectronics\STM32Cube\STM32CubeProgrammer\bin"),
        PathBuf::from("/Applications/STMicroelectronics/STM32Cube/STM32CubeProgrammer/STM32CubeProgrammer.app/Contents/MacOs/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        places
            .push(PathBuf::from(home).join("STMicroelectronics/STM32Cube/STM32CubeProgrammer/bin"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        places.extend(std::env::split_paths(&path));
    }
    places.into_iter().find_map(|dir| {
        ["STM32_Programmer_CLI.exe", "STM32_Programmer_CLI"]
            .iter()
            .map(|name| dir.join(name))
            .find(|p| p.is_file())
    })
}

/// The board's external loader, next to the programmer.
fn loader_next_to(cli: &Path) -> Option<PathBuf> {
    let dir = cli.parent()?.join("ExternalLoader");
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.contains("STM32H735G-DK") && name.ends_with(".stldr")
        })
        .collect();
    found.sort();
    found
        .iter()
        .find(|p| p.to_string_lossy().contains("MX25LM51245G"))
        .or(found.first())
        .cloned()
}

/// `cargo proj mbedded photos <folder>`.
pub fn photos(folder: &Path, no_flash: bool, loader: Option<PathBuf>) -> Result<()> {
    let mut files = photo_files(folder)?;
    if files.is_empty() {
        return Err(format!(
            "no .jpg, .jpeg or .png files in {}",
            folder.display()
        ));
    }
    if files.len() > MAX_PHOTOS {
        println!(
            "{} photos found; the board uses the first {MAX_PHOTOS}.",
            files.len()
        );
        files.truncate(MAX_PHOTOS);
    }
    let mut images = Vec::with_capacity(files.len());
    for f in &files {
        let img = image::open(f).map_err(|e| format!("cannot open {}: {e}", f.display()))?;
        println!(
            "  {}",
            f.file_name().and_then(|n| n.to_str()).unwrap_or("?")
        );
        images.push(img.to_rgb8());
    }
    let bytes = album(&images);
    let out = root().join("target").join("album.bin");
    std::fs::write(&out, &bytes).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    println!(
        "\nalbum: {} photos, {} -> {}",
        images.len(),
        kb(bytes.len() as u64),
        out.display()
    );

    let cli = programmer();
    let loader = loader.or_else(|| cli.as_deref().and_then(loader_next_to));
    let (Some(cli), Some(loader), false) = (cli.clone(), loader.clone(), no_flash) else {
        println!("\nTo write it to the board's flash, install STM32CubeProgrammer and run:");
        println!(
            "  STM32_Programmer_CLI -c port=SWD mode=UR -el <ExternalLoader>/MX25LM51245G_STM32H735G-DK.stldr -d {} {FLASH_ADDRESS} -v -rst",
            out.display()
        );
        if cli.is_some() && loader.is_none() {
            println!("(STM32CubeProgrammer was found, but not the STM32H735G-DK external loader; pass it with --loader)");
        }
        return Ok(());
    };
    let mut c = std::process::Command::new(cli);
    c.args(["-c", "port=SWD", "mode=UR", "-el"])
        .arg(&loader)
        .arg("-d")
        .arg(&out)
        .args([FLASH_ADDRESS, "-v", "-rst"]);
    exec(c)?;
    println!("\nThe album is on the board. It shows on the celebration screen.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, rgb: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(w, h, image::Rgb(rgb))
    }

    #[test]
    fn the_album_header_matches_the_firmware() {
        let a = album(&[flat(10, 10, [0, 0, 0])]);
        assert_eq!(&a[..8], b"H7ALBUM1");
        assert_eq!(a[8..12], 1u32.to_le_bytes());
        assert_eq!(a[12..16], 128u32.to_le_bytes());
        assert!(a[16..64].iter().all(|&b| b == 0));
        assert_eq!(a.len(), 64 + 128 * 128 * 2);
    }

    #[test]
    fn photos_are_squared_and_shrunk_from_the_middle() {
        let mut wide = flat(400, 100, [0, 0, 255]);
        for y in 0..100 {
            for x in 150..250 {
                wide.put_pixel(x, y, image::Rgb([255, 0, 0]));
            }
        }
        let sq = square(&wide);
        assert_eq!(sq.dimensions(), (128, 128));
        assert_eq!(sq.get_pixel(64, 64).0, [255, 0, 0], "the middle is kept");
    }

    #[test]
    fn pure_colors_stay_exact_and_pixels_are_low_byte_first() {
        let a = album(&[flat(128, 128, [255, 0, 0])]);
        assert_eq!(a[64..66], 0xF800u16.to_le_bytes());
        let px = dither_rgb565(&flat(4, 4, [255, 255, 255]));
        assert!(px.iter().all(|&v| v == 0xFFFF));
    }

    #[test]
    fn dithering_mixes_neighbors_for_in_between_colors() {
        // 0x84 sits between two 5-bit red levels: dithering uses both.
        let px = dither_rgb565(&flat(16, 16, [0x84, 0, 0]));
        let reds: std::collections::BTreeSet<u16> = px.iter().map(|v| v >> 11).collect();
        assert!(reds.len() >= 2, "{reds:?}");
    }
}
