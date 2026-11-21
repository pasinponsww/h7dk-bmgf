/* STM32H735IG: 1 MB flash, 128 KB DTCM, 320 KB AXI SRAM.

   RAM is the DTCM, wired straight to the core: variables and the stack live
   there, so the CPU does not fight the debugger or the LCD controller for the
   main bus while it draws. The DTCM is also the fastest RAM on the chip.

   AXISRAM holds what the LCD controller reads (it cannot reach the DTCM):
   the framebuffer, placed there with #[link_section = ".axisram.<name>"].
   This section is not cleared at start-up; the code that owns it clears it.

   No "INSERT AFTER .bss" below on purpose: cortex-m-rt would then count this
   section as part of .bss and its start-up code would try to clear from the
   DTCM all the way to the end of the framebuffer, running off the end of the
   DTCM (a bus fault before main). */
MEMORY
{
  FLASH   : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM     : ORIGIN = 0x20000000, LENGTH = 128K
  AXISRAM : ORIGIN = 0x24000000, LENGTH = 320K
}

SECTIONS
{
  .axisram (NOLOAD) : ALIGN(32)
  {
    *(.axisram .axisram.*);
    . = ALIGN(32);
  } > AXISRAM
}

/* The start-up code clears .bss; it must stay inside the DTCM. */
ASSERT(__ebss <= ORIGIN(RAM) + LENGTH(RAM), "
ERROR(memory.x): .bss runs past the end of the DTCM (RAM). Do not INSERT the .axisram section after .bss.");
