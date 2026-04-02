OUTPUT_ARCH(riscv)
ENTRY(_start)
SECTIONS {
  . = 0x100000;
  .text : {
    *(.text.entry)
    *(.text .text.*)
  }
  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  }
  .data : {
    *(.data .data.*)
    *(.sdata .sdata.*)
  }
  .bss : {
    *(.bss .bss.*)
    *(.sbss .sbss.*)
    . = ALIGN(16);
    __heap_start = .;
    . += 0x01000000;
    __heap_end = .;
  }
}
