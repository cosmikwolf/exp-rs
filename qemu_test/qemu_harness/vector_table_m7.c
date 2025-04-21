#include <stdint.h>
#include "qemu_test_harness.h"

/* Cortex-M7 vector table and startup code */

void Reset_Handler(void);
void Default_Handler(void);
void HardFault_Handler(void);
void MemManage_Handler(void);
void BusFault_Handler(void);
void UsageFault_Handler(void);
void DebugMon_Handler(void);

/* FreeRTOS interrupt handlers */
#ifdef USE_FREERTOS
extern void SVC_Handler(void);
extern void PendSV_Handler(void);
extern void SysTick_Handler(void);
#endif

/* Symbols defined by linker script */
extern uint32_t _sidata; /* Start of init values in flash */
extern uint32_t _sdata;  /* Start of .data in RAM */
extern uint32_t _edata;  /* End of .data in RAM */
extern uint32_t _sbss;   /* Start of .bss in RAM */
extern uint32_t _ebss;   /* End of .bss in RAM */
extern unsigned long _estack;  /* Top of stack from linker script */

/* Vector table */
__attribute__ ((section(".isr_vector")))
void (* const g_pfnVectors[])(void) = {
    (void (*)(void))(&_estack), /* Initial stack pointer from linker script */
    Reset_Handler,                  /* Reset handler */
    Default_Handler,                /* NMI handler */
    HardFault_Handler,              /* Hard fault handler */
    MemManage_Handler,              /* Memory management fault */
    BusFault_Handler,               /* Bus fault */
    UsageFault_Handler,             /* Usage fault */
    0, 0, 0, 0,                     /* Reserved */
#ifdef USE_FREERTOS
    SVC_Handler,                        /* SVCall (FreeRTOS) */
    DebugMon_Handler,                   /* Debug monitor */
    0,                                 /* Reserved */
    PendSV_Handler,                     /* PendSV (FreeRTOS) */
    SysTick_Handler,                    /* SysTick (FreeRTOS) */
#else
    Default_Handler,                /* SVCall */
    DebugMon_Handler,               /* Debug monitor */
    0,                              /* Reserved */
    Default_Handler,                /* PendSV */
    Default_Handler,                /* SysTick */
#endif
};

/* Default interrupt handler */
void Default_Handler(void) {
    qemu_print("!!! Default_Handler triggered !!!\n");
    qemu_exit(1);
    while(1);
}

void HardFault_Handler(void) {
    uint32_t *sp;
    uint32_t exc_lr;
    __asm volatile ("mov %0, lr" : "=r" (exc_lr));
    __asm volatile ("mrs %0, msp" : "=r" (sp));

    register void *cur_sp asm("sp");
    qemu_print("!!! HardFault_Handler triggered !!!\n");
    qemu_printf("SP  = %p\n", cur_sp);

    // Check if SP is valid (in RAM range)
    if ((uint32_t)sp >= 0x20000000 && (uint32_t)sp < 0x20400000) {
        uint32_t r0 = sp[0];
        uint32_t r1 = sp[1];
        uint32_t r2 = sp[2];
        uint32_t r3 = sp[3];
        uint32_t r12 = sp[4];
        uint32_t lr = sp[5];
        uint32_t pc = sp[6];
        uint32_t psr = sp[7];

        qemu_printf("R0  = 0x%08x\n", r0);
        qemu_printf("R1  = 0x%08x\n", r1);
        qemu_printf("R2  = 0x%08x\n", r2);
        qemu_printf("R3  = 0x%08x\n", r3);
        qemu_printf("R12 = 0x%08x\n", r12);
        qemu_printf("LR  = 0x%08x\n", lr);
        qemu_printf("PC  = 0x%08x\n", pc);
        qemu_printf("xPSR= 0x%08x\n", psr);

        // Optional: dump a few words from the stack
        qemu_print("Stack dump:\n");
        for (int i = 0; i < 16; ++i) {
            qemu_printf("  [%02d] 0x%08x\n", i, ((uint32_t*)sp)[i]);
        }
    } else {
        qemu_printf("SP  = %p (invalid stack pointer, not in RAM)\n", sp);
    }

    // Print EXC_LR
    qemu_printf("EXC_LR = 0x%08x\n", exc_lr);

    // Print fault status registers
    volatile uint32_t cfsr = *((volatile uint32_t *)0xE000ED28);
    volatile uint32_t hfsr = *((volatile uint32_t *)0xE000ED2C);
    volatile uint32_t mmfar = *((volatile uint32_t *)0xE000ED34);
    volatile uint32_t bfar = *((volatile uint32_t *)0xE000ED38);

    qemu_printf("CFSR = 0x%08x\n", cfsr);
    qemu_printf("HFSR = 0x%08x\n", hfsr);
    qemu_printf("MMFAR = 0x%08x\n", mmfar);
    qemu_printf("BFAR = 0x%08x\n", bfar);

    // Print CONTROL, MSP, PSP
    uint32_t control, msp, psp;
    __asm volatile ("mrs %0, control" : "=r" (control));
    __asm volatile ("mrs %0, msp" : "=r" (msp));
    __asm volatile ("mrs %0, psp" : "=r" (psp));
    qemu_printf("CONTROL = 0x%08x, MSP = 0x%08x, PSP = 0x%08x\n", control, msp, psp);

    qemu_exit(1);
    while(1);
}

void MemManage_Handler(void) {
    qemu_print("!!! MemManage Fault triggered !!!\n");
    qemu_exit(1);
    while(1);
}

void BusFault_Handler(void) {
    qemu_print("!!! BusFault triggered !!!\n");
    qemu_exit(1);
    while(1);
}

void UsageFault_Handler(void) {
    qemu_print("!!! UsageFault triggered !!!\n");
    qemu_exit(1);
    while(1);
}

void DebugMon_Handler(void) {
    qemu_print("!!! DebugMon_Handler triggered !!!\n");
    qemu_exit(1);
    while(1);
}

/* Reset handler */
extern int main(void);
void Reset_Handler(void) {
    // Enable FPU (set CP10 and CP11 full access)
    #define SCB_CPACR (*(volatile uint32_t *)0xE000ED88)
    SCB_CPACR |= (0xF << 20);
    __asm volatile ("dsb");
    __asm volatile ("isb");

    uint32_t *src, *dst;

    /* Copy data section from flash (_sidata) to RAM (_sdata to _edata) */
    src = &_sidata;
    dst = &_sdata;
    while (dst < &_edata) {
        *dst++ = *src++;
    }

    /* Zero initialize the .bss section (_sbss to _ebss) */
    dst = &_sbss;
    while (dst < &_ebss) {
        *dst++ = 0;
    }

    /* Call newlib/libc init array for C++/libc constructors */
    extern void __libc_init_array(void);
    __libc_init_array();

    /* Call main */
    main();

    /* If main returns, loop forever */
    while(1);
}
