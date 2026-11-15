//! The firmware and the board: finding the board through probe-rs, and
//! measuring how much of the chip the firmware uses.

use std::path::PathBuf;
use std::process::Command;

use super::console::kb;
use super::paths::{ensure_elf, root};
use super::process::{capture, tool, Result};

/// Shown when no board is connected.
pub const NO_BOARD: &str = "no board found: probe-rs sees no debug probe.\n  \
    - Plug the board's ST-LINK USB port into the PC (the port labelled ST-LINK, not the user USB).\n  \
    - Try another USB cable, since some cables only charge.\n  \
    - Check again with: cargo proj common status";

/// Picks the first probe out of `probe-rs list` output. `None` when there is none.
pub fn parse_probe_list(out: &str) -> Option<String> {
    if out.contains("No debug probes") {
        return None;
    }
    out.lines()
        .map(str::trim)
        .find(|l| l.starts_with('['))
        .map(str::to_string)
}

/// True if probe-rs is installed.
pub fn probe_rs_installed() -> bool {
    let mut c = tool("probe-rs");
    c.arg("--version");
    capture(c).is_ok()
}

/// The connected probe, if any.
pub fn probe() -> Result<Option<String>> {
    if !probe_rs_installed() {
        return Err("probe-rs is not installed. Run: cargo proj common setup".to_string());
    }
    let mut c = tool("probe-rs");
    c.arg("list");
    Ok(parse_probe_list(&capture(c).unwrap_or_default()))
}

/// The connected probe, or an error that says how to connect the board.
pub fn require_probe() -> Result<String> {
    probe()?.ok_or_else(|| NO_BOARD.to_string())
}

/// Section sizes of the firmware, in bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Sizes {
    /// Code and read-only data.
    pub text: u64,
    /// Initialised data.
    pub data: u64,
    /// Zero-initialised data.
    pub bss: u64,
}

impl Sizes {
    /// Bytes stored in flash.
    pub fn flash(&self) -> u64 {
        self.text + self.data
    }
    /// Bytes used in RAM at run time.
    pub fn ram(&self) -> u64 {
        self.data + self.bss
    }
}

/// Reads the numbers from `llvm-size` output (text, data, bss ...).
pub fn parse_size_output(out: &str) -> Option<Sizes> {
    let line = out.lines().filter(|l| !l.trim().is_empty()).nth(1)?;
    let mut n = line.split_whitespace().map(|v| v.parse::<u64>().ok());
    Some(Sizes {
        text: n.next()??,
        data: n.next()??,
        bss: n.next()??,
    })
}

/// Reads "LENGTH = 1024K" for a region of `memory.x`.
fn region_length(memory_x: &str, region: &str) -> Option<u64> {
    let line = memory_x
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(region))?;
    let value = line.split("LENGTH").nth(1)?.trim_start_matches([' ', '=']);
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().ok()?;
    let unit = value.chars().nth(digits.len())?;
    Some(match unit {
        'K' | 'k' => n * 1024,
        'M' | 'm' => n * 1024 * 1024,
        _ => n,
    })
}

/// Flash and RAM size of the chip, read from `memory.x` so there is one source of truth.
pub fn memory_limits(memory_x: &str) -> Option<(u64, u64)> {
    Some((
        region_length(memory_x, "FLASH")?,
        region_length(memory_x, "RAM")?,
    ))
}

fn llvm_size() -> Option<PathBuf> {
    let mut c = tool("rustc");
    c.args(["--print", "sysroot"]);
    let sysroot = capture(c).ok()?;
    let mut c = tool("rustc");
    c.arg("-vV");
    let host = capture(c)
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("host: ").map(str::to_string))?;
    let bin = PathBuf::from(sysroot)
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin");
    ["llvm-size.exe", "llvm-size"]
        .iter()
        .map(|f| bin.join(f))
        .find(|p| p.is_file())
}

/// Measures the firmware ELF. Fails if there is no build or no `llvm-size`.
pub fn measure(release: bool) -> Result<Sizes> {
    let elf = ensure_elf(release)?;
    let program = llvm_size().ok_or("llvm-size not found")?;
    let mut c = Command::new(program);
    c.arg(&elf);
    parse_size_output(&capture(c)?).ok_or_else(|| "could not read the size report".to_string())
}

/// Prints flash and RAM use as a share of the chip.
pub fn print_sizes(s: &Sizes) {
    let memory_x = std::fs::read_to_string(root().join("memory.x")).unwrap_or_default();
    let (flash_max, ram_max) = memory_limits(&memory_x).unwrap_or((1024 * 1024, 320 * 1024));
    let pct = |used: u64, max: u64| used as f64 * 100.0 / max as f64;
    println!(
        "\n  flash  {:>10} of {} ({:.1}%)\n  ram    {:>10} of {} ({:.1}%)",
        kb(s.flash()),
        kb(flash_max),
        pct(s.flash(), flash_max),
        kb(s.ram()),
        kb(ram_max),
        pct(s.ram(), ram_max),
    );
}

#[cfg(test)]
mod tests {
    use super::{memory_limits, parse_probe_list, parse_size_output, Sizes, NO_BOARD};
    use crate::tools::paths::root;

    #[test]
    fn probe_list_with_no_probe_is_none() {
        assert_eq!(parse_probe_list("No debug probes were found."), None);
        assert_eq!(parse_probe_list(""), None);
    }

    #[test]
    fn probe_list_picks_the_first_probe() {
        let out = "The following debug probes were found:\n[0]: STLink V3 -- 0483:374e:001 (ST-LINK)\n[1]: other\n";
        assert_eq!(
            parse_probe_list(out).as_deref(),
            Some("[0]: STLink V3 -- 0483:374e:001 (ST-LINK)")
        );
    }

    #[test]
    fn the_no_board_message_tells_the_user_what_to_do() {
        assert!(NO_BOARD.contains("ST-LINK"));
        assert!(NO_BOARD.contains("cargo proj common status"));
    }

    #[test]
    fn size_output_is_parsed() {
        let out = "   text\t   data\t    bss\t    dec\t    hex\tfilename\n  12345\t    100\t   2000\t  14445\t   386d\tfile\n";
        let s = parse_size_output(out).unwrap();
        assert_eq!(
            s,
            Sizes {
                text: 12345,
                data: 100,
                bss: 2000
            }
        );
        assert_eq!(s.flash(), 12445);
        assert_eq!(s.ram(), 2100);
    }

    #[test]
    fn garbage_size_output_is_refused() {
        assert!(parse_size_output("garbage").is_none());
        assert!(parse_size_output("").is_none());
        assert!(parse_size_output("text data bss\nabc def ghi\n").is_none());
    }

    #[test]
    fn memory_limits_come_from_memory_x() {
        let m = "MEMORY\n{\n  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K\n  RAM   : ORIGIN = 0x24000000, LENGTH = 320K\n}\n";
        assert_eq!(memory_limits(m), Some((1024 * 1024, 320 * 1024)));
        assert_eq!(memory_limits("nothing here"), None);
    }

    #[test]
    fn megabyte_units_are_understood() {
        let m = "FLASH : ORIGIN = 0, LENGTH = 2M\nRAM : ORIGIN = 0, LENGTH = 64K\n";
        assert_eq!(memory_limits(m), Some((2 * 1024 * 1024, 64 * 1024)));
    }

    #[test]
    fn the_real_memory_x_parses_to_the_chip_sizes() {
        let text = std::fs::read_to_string(root().join("memory.x")).unwrap();
        assert_eq!(memory_limits(&text), Some((1024 * 1024, 320 * 1024)));
    }
}
