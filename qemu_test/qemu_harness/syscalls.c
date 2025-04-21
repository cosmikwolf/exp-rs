#include <stdint.h>

void _exit(int status)
{
    // Call QEMU semihosting exit with the status code
    register unsigned int r0 __asm__("r0") = 0x18;       // SYS_EXIT
    register unsigned int r1 __asm__("r1") = (unsigned int)status;  // exit code
    __asm__ __volatile__(
        "bkpt #0xAB"
        : "+r" (r0), "+r" (r1)
        :
        :
    );
    while(1); // Should never return
}
