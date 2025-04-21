/* Linker script for QEMU MPS2 AN500 Cortex-M7 */

MEMORY
{
  FLASH (rx)      : ORIGIN = 0x00000000, LENGTH = 4M
  FREERTOS_HEAP   : ORIGIN = 0x20000000, LENGTH = 128K
  MALLOC_HEAP     : ORIGIN = 0x20020000, LENGTH = 128K
  RAM             : ORIGIN = 0x20040000, LENGTH = 0x3C0000  /* rest of RAM after heaps (3.75MB - 256KB) */
}

SECTIONS
{
  .isr_vector : {
    KEEP(*(.isr_vector))
  } > FLASH

  .text : {
    *(.text*)
    *(.rodata*)
    *(.glue_7)
    *(.glue_7t)
    *(.eh_frame)
  } > FLASH

  /* C++ init/fini sections for newlib/rdimon */
  .init : {
    KEEP(*(.init))
  } > FLASH

  .fini : {
    KEEP(*(.fini))
  } > FLASH

  .init_array : {
    PROVIDE_HIDDEN(__init_array_start = .);
    KEEP(*(.init_array*))
    PROVIDE_HIDDEN(__init_array_end = .);
  } > FLASH

  .fini_array : {
    PROVIDE_HIDDEN(__fini_array_start = .);
    KEEP(*(.fini_array*))
    PROVIDE_HIDDEN(__fini_array_end = .);
  } > FLASH

  .ARM.exidx : {
    __exidx_start = .;
    *(.ARM.exidx*)
    __exidx_end = .;
  } > FLASH

  /* After .ARM.exidx, align to 4 bytes and mark end of FLASH image */
  . = ALIGN(4);
  _flash_end = .;

  /* FreeRTOS heap section (128 KB) */
  .freertos_heap (NOLOAD) : {
    . = ALIGN(8);
    __freertos_heap_start = .;
    . = . + 0x20000; /* 128 KB */
    __freertos_heap_end = .;
  } > FREERTOS_HEAP

  /* newlib malloc heap section (128 KB) */
  .malloc_heap (NOLOAD) : {
    . = ALIGN(8);
    __malloc_heap_start = .;
    . = . + 0x20000; /* 128 KB */
    __malloc_heap_end = .;
  } > MALLOC_HEAP

  /* Initialized data section in RAM, load from FLASH */
  .data : AT ( _flash_end ) {
    _sdata = .;
    *(.data*)
    _edata = .;
  } > RAM

  /* Init values for .data section stored in FLASH */
  _sidata = LOADADDR(.data);

  /* Reserve 16 KB at top of RAM for stack */
  _estack = ORIGIN(RAM) + LENGTH(RAM);
  _stack_limit = _estack - 0x8000; /* 32 KB stack */

  /* Uninitialized data section (zero-initialized) */
  .bss : {
    _sbss = .;
    *(.bss*)
    *(COMMON)
    _ebss = .;
  } > RAM

  /* Assert that .bss does not overlap reserved stack */
  ASSERT(_ebss < _stack_limit, "ERROR: .bss overlaps reserved stack!")

}

/* Define end symbols for _sbrk() */
PROVIDE(end = _ebss);
PROVIDE(_end = _ebss);

/* Define initial stack pointer symbol (top of RAM) */
_estack = ORIGIN(RAM) + LENGTH(RAM);
_stack_limit = _estack - 0x4000;
