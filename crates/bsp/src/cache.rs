//! The Cortex-M7 caches.
//!
//! The data cache speeds up everything the CPU reads from flash: fonts,
//! icons, the story text and the code's constant tables. It must not hold
//! the framebuffer, though: the LCD controller reads that straight from the
//! AXI SRAM, so pixels the CPU wrote could sit in the cache and never reach
//! the screen. The MPU marks the AXI SRAM as normal memory that is never
//! cached, then the caches are switched on.
//!
//! Variables, the stack and the log buffer live in the DTCM, which is wired
//! to the core and is never cached, so the debugger always sees them.

use cortex_m::peripheral::{CPUID, MPU, SCB};

/// Start of the AXI SRAM, where the framebuffer is.
const AXI_SRAM: u32 = 0x2400_0000;
/// MPU region size field: 2^(18 + 1) bytes = 512 KB, which covers the
/// 320 KB AXI SRAM (MPU regions are powers of two, aligned to their size).
const SIZE_512K: u32 = 18;

// MPU_RASR fields (ARMv7-M architecture reference manual, B3.5.9).
const RASR_ENABLE: u32 = 1;
const RASR_SIZE_SHIFT: u32 = 1;
/// TEX = 001, C = 0, B = 0: normal memory, not cacheable.
const RASR_NORMAL_UNCACHED: u32 = 0b001 << 19;
/// Read and write for everyone.
const RASR_AP_FULL: u32 = 0b011 << 24;
/// No code runs from it.
const RASR_XN: u32 = 1 << 28;

// MPU_CTRL fields.
const CTRL_ENABLE: u32 = 1;
/// Everything without a region keeps the default memory map.
const CTRL_PRIVDEFENA: u32 = 1 << 2;

/// Marks the AXI SRAM as never cached, then switches on the instruction and
/// data caches.
#[allow(unsafe_code)]
pub fn enable(scb: &mut SCB, cpuid: &mut CPUID, mpu: &mut MPU) {
    cortex_m::asm::dmb();
    // SAFETY: the MPU is off while region 0 is written, the region only
    // makes the AXI SRAM uncached (the safe choice for memory the LCD
    // controller and DMA read), and every address without a region keeps the
    // default map, so no access that worked before faults now.
    unsafe {
        mpu.ctrl.write(0);
        mpu.rnr.write(0);
        mpu.rbar.write(AXI_SRAM);
        mpu.rasr.write(
            RASR_XN
                | RASR_AP_FULL
                | RASR_NORMAL_UNCACHED
                | SIZE_512K << RASR_SIZE_SHIFT
                | RASR_ENABLE,
        );
        mpu.ctrl.write(CTRL_PRIVDEFENA | CTRL_ENABLE);
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();

    scb.enable_icache();
    scb.enable_dcache(cpuid);
}
