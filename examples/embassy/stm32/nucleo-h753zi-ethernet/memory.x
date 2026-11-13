/* NUCLEO-H753ZI: 2 MiB flash. All application RAM is placed in D2 SRAM
 * (SRAM1/2/3), accessible by Ethernet DMA. DTCM is deliberately not used.
 * Keep the Cortex-M7 data cache disabled unless DMA coherency is provided. */
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 2048K
    RAM   : ORIGIN = 0x30000000, LENGTH = 288K
}
