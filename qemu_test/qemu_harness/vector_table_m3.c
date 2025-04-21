#include <stdint.h>

/* Minimal vector table for Cortex-M3 */
void Reset_Handler(void);
void Default_Handler(void);

/* The vector table */
__attribute__ ((section(".isr_vector")))
void (* const g_pfnVectors[])(void) = {
    (void (*)(void))((uint32_t)0x20000000 + 0x10000), /* Initial stack pointer */
    Reset_Handler,                  /* Reset handler */
    Default_Handler,                /* NMI handler */
    Default_Handler,                /* Hard fault handler */
    Default_Handler,                /* Memory management fault */
    Default_Handler,                /* Bus fault */
    Default_Handler,                /* Usage fault */
    0, 0, 0, 0,                     /* Reserved */
    Default_Handler,                /* SVCall */
    Default_Handler,                /* Debug monitor */
    0,                              /* Reserved */
    Default_Handler,                /* PendSV */
    Default_Handler,                /* SysTick */
};

/* Default handler for all interrupts */
void Default_Handler(void) {
    while(1);
}

/* Reset handler - calls main() */
extern int main(void);
void Reset_Handler(void) {
    /* Initialize data and bss sections */
    extern uint32_t _sdata, _edata, _sbss, _ebss;
    uint32_t *src, *dst;

    /* Copy data section from flash to RAM */
    src = (uint32_t*)&_edata;
    dst = (uint32_t*)&_sdata;
    while(dst < (uint32_t*)&_edata) {
        *dst++ = *src++;
    }

    /* Clear bss section */
    dst = (uint32_t*)&_sbss;
    while(dst < (uint32_t*)&_ebss) {
        *dst++ = 0;
    }

    /* Call main */
    main();

    /* If main returns, loop forever */
    while(1);
}
