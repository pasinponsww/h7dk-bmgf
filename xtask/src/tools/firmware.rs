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

/// How much of each memory the firmware uses, in bytes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    /// Flash: code, constants, and the start values of variables.
    pub flash: u64,
    /// DTCM (`RAM` in `memory.x`): variables. The stack gets the rest.
    pub ram: u64,
    /// AXI SRAM: what the LCD controller reads, the framebuffer.
    pub axi: u64,
}

/// Reads the section table of `llvm-size -A` ("section size addr" rows).
pub fn parse_size_output(out: &str) -> Option<Sizes> {
    let mut sizes = Sizes::default();
    let mut seen = false;
    for line in out.lines() {
        let mut cols = line.split_whitespace();
        let (Some(name), Some(size)) = (cols.next(), cols.next()) else {
            continue;
        };
        let Ok(size) = size.parse::<u64>() else {
            continue;
        };
        match name {
            ".vector_table" | ".text" | ".rodata" | ".gnu.sgstubs" => sizes.flash += size,
            ".data" => {
                sizes.flash += size;
                sizes.ram += size;
            }
            ".bss" | ".uninit" => sizes.ram += size,
            n if n.starts_with(".axisram") => sizes.axi += size,
            _ => continue,
        }
        seen = true;
    }
    seen.then_some(sizes)
}

/// Reads "LENGTH = 1024K" for a region of `memory.x`.
fn region_length(memory_x: &str, region: &str) -> Option<u64> {
    let line = memory_x.lines().map(str::trim).find(|l| {
        l.strip_prefix(region)
            .is_some_and(|rest| rest.trim_start().starts_with(':'))
    })?;
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

/// Size of each memory of the chip, read from `memory.x` so there is one source of truth.
#[derive(Debug, PartialEq, Eq)]
pub struct Limits {
    /// Flash.
    pub flash: u64,
    /// DTCM.
    pub ram: u64,
    /// AXI SRAM, or 0 when `memory.x` has no `AXISRAM` region.
    pub axi: u64,
}

/// Reads the memory sizes from `memory.x`.
pub fn memory_limits(memory_x: &str) -> Option<Limits> {
    Some(Limits {
        flash: region_length(memory_x, "FLASH")?,
        ram: region_length(memory_x, "RAM")?,
        axi: region_length(memory_x, "AXISRAM").unwrap_or(0),
    })
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
    c.arg("-A").arg(&elf);
    parse_size_output(&capture(c)?).ok_or_else(|| "could not read the size report".to_string())
}

/// Prints flash and RAM use as a share of the chip.
pub fn print_sizes(s: &Sizes) {
    let memory_x = std::fs::read_to_string(root().join("memory.x")).unwrap_or_default();
    let max = memory_limits(&memory_x).unwrap_or(Limits {
        flash: 1024 * 1024,
        ram: 128 * 1024,
        axi: 320 * 1024,
    });
    let row = |name: &str, used: u64, max: u64, what: &str| {
        let pct = used as f64 * 100.0 / max.max(1) as f64;
        println!(
            "  {name:<8} {:>10} of {} ({pct:.1}%)  {what}",
            kb(used),
            kb(max)
        );
    };
    println!();
    row("flash", s.flash, max.flash, "code and text");
    row("ram", s.ram, max.ram, "variables; the stack gets the rest");
    if s.axi > 0 || max.axi > 0 {
        row("lcd ram", s.axi, max.axi, "framebuffer");
    }
}

#[cfg(test)]
mod tests {
    use super::{memory_limits, parse_probe_list, parse_size_output, Limits, Sizes, NO_BOARD};
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
        let out = "h7dk-fw  :
section           size         addr
.vector_table      716    134217728
.text           33016    134218444
.rodata         13764    134251460
.data            3536    536870912
.bss            15648    536874448
.axisram       261120    603979776
.uninit          1120    536890096
.debug_info    412934            0
Total          741854
";
        let s = parse_size_output(out).unwrap();
        assert_eq!(
            s,
            Sizes {
                flash: 716 + 33016 + 13764 + 3536,
                ram: 3536 + 15648 + 1120,
                axi: 261120,
            }
        );
    }

    #[test]
    fn garbage_size_output_is_refused() {
        assert!(parse_size_output("garbage").is_none());
        assert!(parse_size_output("").is_none());
        assert!(parse_size_output(
            "section size addr
.debug_info abc 0
"
        )
        .is_none());
    }

    #[test]
    fn memory_limits_come_from_memory_x() {
        let m = "MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
  AXISRAM : ORIGIN = 0x24000000, LENGTH = 320K
}
";
        assert_eq!(
            memory_limits(m),
            Some(Limits {
                flash: 1024 * 1024,
                ram: 128 * 1024,
                axi: 320 * 1024
            })
        );
        assert_eq!(memory_limits("nothing here"), None);
    }

    #[test]
    fn megabyte_units_are_understood_and_axisram_is_optional() {
        let m = "FLASH : ORIGIN = 0, LENGTH = 2M
RAM : ORIGIN = 0, LENGTH = 64K
";
        assert_eq!(
            memory_limits(m),
            Some(Limits {
                flash: 2 * 1024 * 1024,
                ram: 64 * 1024,
                axi: 0
            })
        );
    }

    #[test]
    fn the_real_memory_x_parses_to_the_chip_sizes() {
        let text = std::fs::read_to_string(root().join("memory.x")).unwrap();
        assert_eq!(
            memory_limits(&text),
            Some(Limits {
                // The last 128 KB sector keeps saved progress.
                flash: 896 * 1024,
                ram: 128 * 1024,
                axi: 320 * 1024
            })
        );
    }
}
