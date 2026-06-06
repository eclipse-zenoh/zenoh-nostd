MEMORY
{
  FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 256K
  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 64K
}

/* This is provided by cortex-m-rt's link.x */
STACK_SIZE = 0x2000;

/* This provides the heap start for alloc-cortex-m */
__heap_start = ORIGIN(RAM) + LENGTH(RAM) - 0x2000;
__heap_end = ORIGIN(RAM) + LENGTH(RAM);