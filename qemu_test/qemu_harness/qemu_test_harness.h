#ifndef QEMU_TEST_HARNESS_H
#define QEMU_TEST_HARNESS_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <stdint.h>

// Test result codes
typedef enum { TEST_PASS = 0, TEST_FAIL = 1, TEST_SKIP = 2 } test_result_t;

// Test function type
typedef test_result_t (*test_func_t)(void);

// Test case structure
typedef struct {
    const char *name;
    test_func_t func;
} test_case_t;

/* QEMU semihosting functions */
void qemu_print(const char *str);
void qemu_print_int(int value);
void qemu_printf(const char *fmt, ...);
void qemu_exit(int status);

/* Semihosting file operations */
int qemu_file_open(const char *filename, const char *mode);
int qemu_file_close(int fd);
int qemu_file_read(int fd, void *buf, int len);
int qemu_file_write(int fd, const void *buf, int len);

/* Test runner functions */
int run_tests(const test_case_t *tests, int num_tests);
void test_assert(int condition, const char *message);

/* Timing functions */
void init_hardware_timer(void);  /* Primary function - initializes CMSDK Dual Timer */
void init_dwt_counter(void);     /* Legacy function, calls init_hardware_timer */
void benchmark_start(void);
void check_counter_rollover(void);
uint32_t benchmark_stop(void);
uint32_t qemu_get_tick_count(void);

#ifdef __cplusplus
}
#endif

#endif // QEMU_TEST_HARNESS_H
