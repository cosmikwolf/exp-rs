#include <sys/types.h>
#include <stdint.h>
#include "qemu_test_harness.h"

extern char __malloc_heap_start;
extern char __malloc_heap_end;

static char *heap_end = 0;

void * _sbrk(ptrdiff_t incr) {
    qemu_print("[_sbrk] called\n");
    if (heap_end == 0) {
        heap_end = &__malloc_heap_start;
        qemu_printf("[_sbrk] heap_end initialized to %p\n", heap_end);
    }
    char *prev_heap_end = heap_end;
    char *new_heap_end = heap_end + incr;

    qemu_printf("[_sbrk] called: incr=%d, prev_heap_end=%p, new_heap_end=%p, __malloc_heap_end=%p\n",
                (int)incr, prev_heap_end, new_heap_end, &__malloc_heap_end);

    if (new_heap_end > &__malloc_heap_end) {
        qemu_print("[_sbrk] Out of malloc heap memory!\n");
        return (void *)-1;
    }

    heap_end = new_heap_end;
    return (void *)prev_heap_end;
}
