#include "qemu_test_harness.h"
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

void qemu_printf(const char *fmt, ...) {
  char buf[256];
  va_list args;
  va_start(args, fmt);
  vsnprintf(buf, sizeof(buf), fmt, args);
  va_end(args);
  qemu_print(buf);
}

// QEMU semihosting for output
void qemu_print(const char *str) {
  __asm__ __volatile__("mov r0, #0x04\n" // SYS_WRITE0
                       "mov r1, %0\n"    // String address
                       "bkpt #0xAB\n"    // Semihosting breakpoint
                       :
                       : "r"(str)
                       : "r0", "r1");
}

void qemu_print_int(int value) {
  char buffer[12];
  int pos = 0;
  int temp = value;

  // Handle negative numbers
  if (value < 0) {
    buffer[pos++] = '-';
    temp = -temp;
  }

  // Convert to string (reverse)
  do {
    buffer[pos++] = '0' + (temp % 10);
    temp /= 10;
  } while (temp > 0);

  // Null terminate
  buffer[pos] = '\0';

  // Reverse the digits
  for (int i = (value < 0 ? 1 : 0), j = pos - 1; i < j; i++, j--) {
    char tmp = buffer[i];
    buffer[i] = buffer[j];
    buffer[j] = tmp;
  }

  qemu_print(buffer);
}

void test_assert(int condition, const char *message) {
  if (!condition) {
    qemu_print("ASSERT FAILED: ");
    qemu_print(message);
    qemu_print("\n");
    qemu_exit(EXIT_FAILURE);
  }
}

int run_tests(const test_case_t *tests, int num_tests) {
  int passed = 0;
  int failed = 0;
  int skipped = 0;

  qemu_print("Starting QEMU tests...\n");

  for (int i = 0; i < num_tests; i++) {
    qemu_print("\nRunning test: ");
    qemu_print(tests[i].name);
    qemu_print("...\n");

    test_result_t result = tests[i].func();

    qemu_print("Finished test: ");
    qemu_print(tests[i].name);
    qemu_print("\n");

    switch (result) {
    case TEST_PASS:
      qemu_print("PASS\n");
      passed++;
      break;
    case TEST_FAIL:
      qemu_print("FAIL\n");
      failed++;
      break;
    case TEST_SKIP:
      qemu_print("SKIP\n");
      skipped++;
      break;
    }
  }

  qemu_print("\nTest Summary:\n");
  qemu_print("Passed: ");
  qemu_print_int(passed);
  qemu_print("\nFailed: ");
  qemu_print_int(failed);
  qemu_print("\nSkipped: ");
  qemu_print_int(skipped);
  qemu_print("\n");

  if (failed > 0) {
    qemu_print("Some tests failed!\n");
  } else {
    qemu_print("All tests completed successfully!\n");
  }

  return failed;
}

void qemu_exit(int status) {
  if (status == EXIT_SUCCESS) {
    exit(0);
  } else {
    register int reg0 __asm__("r0") = 0x18; // angel_SWIreason_ReportException
    register int reg1 __asm__("r1") =
        (0x20026 << 8) | (status & 0xFF); // encode exit status in low byte

    __asm__ __volatile__("bkpt #0xAB" : : "r"(reg0), "r"(reg1) : "memory");
    while (1) {
    }
  }
}

// CMSDK Dual Timer Registers
// Base address for MPS2 AN500 platform
#define CMSDK_TIMER0_BASE 0x40000000
#define CMSDK_TIMER1_BASE 0x40001000

// Timer registers - using Timer1
#define TIMER1_LOAD ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x00))
#define TIMER1_VALUE ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x04))
#define TIMER1_CONTROL ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x08))
#define TIMER1_INTCLR ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x0C))
#define TIMER1_RIS ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x10))
#define TIMER1_MIS ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x14))
#define TIMER1_BGLOAD ((volatile uint32_t *)(CMSDK_TIMER1_BASE + 0x18))

// Timer control register bits
#define TIMER_CTRL_ONESHOT (1 << 0)
#define TIMER_CTRL_32BIT (1 << 1)
#define TIMER_CTRL_DIV1 (0 << 2)
#define TIMER_CTRL_DIV16 (1 << 2)
#define TIMER_CTRL_DIV256 (2 << 2)
#define TIMER_CTRL_IE (1 << 5)
#define TIMER_CTRL_PERIODIC (1 << 6)
#define TIMER_CTRL_ENABLE (1 << 7)

// Global timer state
static uint32_t cycle_start = 0;
int timer_initialized = 0;

// Initialize the CMSDK hardware timer
void init_hardware_timer(void) {
  // Already initialized?
  if (timer_initialized)
    return;

  qemu_print("Initializing CMSDK Timer1 hardware timer for benchmarking...\n");

  // Disable the timer first
  *TIMER1_CONTROL = 0;

  // Clear any pending interrupts
  *TIMER1_INTCLR = 1;

  // Configure as 32-bit timer, no prescaler (DIV1), periodic mode
  uint32_t control = TIMER_CTRL_32BIT | TIMER_CTRL_DIV1 | TIMER_CTRL_PERIODIC;
  *TIMER1_CONTROL = control;

  // Set maximum reload value for maximum range
  *TIMER1_LOAD = 0xFFFFFFFF;

  // Wait for the value to be loaded
  while (*TIMER1_VALUE != 0xFFFFFFFF) {
  }

  // Now enable the timer
  *TIMER1_CONTROL = control | TIMER_CTRL_ENABLE;

  // Memory barriers to ensure all operations are completed
  __asm__ volatile("dmb" ::: "memory");
  __asm__ volatile("dsb" ::: "memory");
  __asm__ volatile("isb" ::: "memory");

  // Test if timer is working
  uint32_t start = *TIMER1_VALUE;

  // Wait a bit
  for (volatile int i = 0; i < 10000; i++) {
  }

  uint32_t end = *TIMER1_VALUE;

  // The timer counts down, so start should be larger
  if (start > end) {
    qemu_printf("CMSDK Timer test: start=%u, end=%u, diff=%u ticks\n", start,
                end, start - end);
    qemu_print("CMSDK Timer initialized successfully!\n");
    timer_initialized = 1;
  } else {
// Try with another base address - some platforms use different addresses
// (Since this is QEMU, we can try a range of possible addresses)
#define TIMER0_ALT_BASE 0x40002000

#undef TIMER1_LOAD
#undef TIMER1_VALUE
#undef TIMER1_CONTROL
#undef TIMER1_INTCLR

#define TIMER1_LOAD ((volatile uint32_t *)(TIMER0_ALT_BASE + 0x00))
#define TIMER1_VALUE ((volatile uint32_t *)(TIMER0_ALT_BASE + 0x04))
#define TIMER1_CONTROL ((volatile uint32_t *)(TIMER0_ALT_BASE + 0x08))
#define TIMER1_INTCLR ((volatile uint32_t *)(TIMER0_ALT_BASE + 0x0C))

    qemu_print(
        "First timer address didn't work, trying alternative address...\n");

    // Disable the timer
    *TIMER1_CONTROL = 0;

    // Clear any pending interrupts
    *TIMER1_INTCLR = 1;

    // Configure as 32-bit timer
    *TIMER1_CONTROL = control;

    // Set reload value
    *TIMER1_LOAD = 0xFFFFFFFF;

    // Enable the timer
    *TIMER1_CONTROL = control | TIMER_CTRL_ENABLE;

    // Memory barriers
    __asm__ volatile("dmb" ::: "memory");
    __asm__ volatile("dsb" ::: "memory");
    __asm__ volatile("isb" ::: "memory");

    // Test again
    start = *TIMER1_VALUE;

    // Wait a bit
    for (volatile int i = 0; i < 10000; i++) {
    }

    end = *TIMER1_VALUE;

    if (start > end) {
      qemu_printf(
          "Alternative CMSDK Timer test: start=%u, end=%u, diff=%u ticks\n",
          start, end, start - end);
      qemu_print("Alternative CMSDK Timer initialized successfully!\n");
      timer_initialized = 1;
    } else {
      qemu_print("ERROR: CMSDK Timer not working at either address.\n");
      qemu_print("Benchmarking requires a working hardware timer.\n");
      qemu_print("ABORTING BENCHMARK.\n");
      qemu_exit(EXIT_FAILURE);
    }
  }

  // Warm up the timer with some sample runs to stabilize it
  qemu_print("Warming up the timer for better stability...\n");
  uint32_t warmup_start, warmup_end, warmup_elapsed;

  // Do a series of short timing operations to warm up the timer
  for (int warmup = 0; warmup < 5; warmup++) {
    // Reset timer to maximum value
    *TIMER1_LOAD = 0xFFFFFFFF;

    // Wait for it to load
    while (*TIMER1_VALUE != 0xFFFFFFFF) {
    }

    // Start timing
    warmup_start = *TIMER1_VALUE;

    // Do some work
    for (volatile int i = 0; i < 100000; i++) {
    }

    // End timing
    warmup_end = *TIMER1_VALUE;

    // Calculate elapsed
    if (warmup_start >= warmup_end) {
      warmup_elapsed = warmup_start - warmup_end;
    } else {
      warmup_elapsed = (0xFFFFFFFF - warmup_end) + warmup_start + 1;
    }

    qemu_printf("Warmup run %d: elapsed=%u ticks\n", warmup + 1,
                warmup_elapsed);

    // Add a small delay between warmup runs
    for (volatile int i = 0; i < 10000; i++) {
    }
  }

  qemu_print("Timer warm-up complete\n");
}

// Start timing measurement
void benchmark_start(void) {
  // Reset timer to ensure consistent measurements
  reset_timer();

  // Memory barriers to ensure proper ordering
  __asm__ volatile("dmb" ::: "memory");
  __asm__ volatile("dsb" ::: "memory");
  __asm__ volatile("isb" ::: "memory");

  // Verify timer is still enabled
  if ((*TIMER1_CONTROL & TIMER_CTRL_ENABLE) == 0) {
    uint32_t control = *TIMER1_CONTROL;
    // Re-enable the timer with existing configuration
    *TIMER1_CONTROL = control | TIMER_CTRL_ENABLE;

    // Memory barriers
    __asm__ volatile("dmb" ::: "memory");
    __asm__ volatile("dsb" ::: "memory");
    __asm__ volatile("isb" ::: "memory");
  }

  // Record the current counter value
  cycle_start = *TIMER1_VALUE;
}

// Reset the timer counter to its maximum value
void reset_timer(void) {

  // Memory barriers before reset
  __asm__ volatile("dmb" ::: "memory");
  __asm__ volatile("dsb" ::: "memory");
  __asm__ volatile("isb" ::: "memory");

  // Writing to LOAD register causes the counter to reload
  *TIMER1_LOAD = 0xFFFFFFFF;

  // Memory barriers after reset
  __asm__ volatile("dmb" ::: "memory");
  __asm__ volatile("dsb" ::: "memory");
  __asm__ volatile("isb" ::: "memory");
}

// Check if timer is working correctly
// Call periodically during long timing operations
void check_counter_rollover(void) {
  // Check if timer is still enabled
  if ((*TIMER1_CONTROL & TIMER_CTRL_ENABLE) == 0) {
    qemu_printf(
        "WARNING: CMSDK Timer disabled during measurement. Re-enabling...\n");

    uint32_t control =
        *TIMER1_CONTROL &
        ~TIMER_CTRL_ENABLE; // Get current config without enable bit
    *TIMER1_CONTROL = control | TIMER_CTRL_ENABLE; // Re-enable

    __asm__ volatile("dmb" ::: "memory");
    __asm__ volatile("dsb" ::: "memory");
    __asm__ volatile("isb" ::: "memory");
  }

  // For CMSDK, verify the counter is still running
  static uint32_t prev_check = 0;
  uint32_t current = *TIMER1_VALUE;

  // First time, just store the value
  if (prev_check == 0) {
    prev_check = current;
    return;
  }

  // Check that the timer is actually decreasing
  // (CMSDK timer counts DOWN)
  if (current >= prev_check) {
    // Timer not decreasing properly
    qemu_printf("WARNING: CMSDK Timer not counting down properly: prev=%u, "
                "current=%u\n",
                prev_check, current);

    // Try to reset the timer
    *TIMER1_CONTROL = 0;       // Disable
    *TIMER1_INTCLR = 1;        // Clear interrupts
    *TIMER1_LOAD = 0xFFFFFFFF; // Reset load value

    // Re-enable with 32-bit, periodic mode
    *TIMER1_CONTROL =
        TIMER_CTRL_32BIT | TIMER_CTRL_PERIODIC | TIMER_CTRL_ENABLE;

    __asm__ volatile("dmb" ::: "memory");
    __asm__ volatile("dsb" ::: "memory");
    __asm__ volatile("isb" ::: "memory");
  }

  prev_check = current;
}

// Stop timing and return elapsed ticks
uint32_t benchmark_stop(void) {
  // Memory barriers to ensure proper ordering
  __asm__ volatile("dmb" ::: "memory");
  __asm__ volatile("dsb" ::: "memory");
  __asm__ volatile("isb" ::: "memory");

  // Read final counter value
  uint32_t end_count = *TIMER1_VALUE;

  // Calculate elapsed ticks
  // CMSDK timer counts DOWN from load value, so calculate difference
  uint32_t elapsed;

  // Since the timer counts down, the start value should be larger than the end
  // value
  if (cycle_start >= end_count) {
    elapsed = cycle_start - end_count;
  } else {
    // Timer wrapped around (full 32-bit counter)
    elapsed = (0xFFFFFFFF - end_count) + cycle_start + 1;
  }

  // Sanity check for very small values
  if (elapsed < 10) {
    qemu_printf("WARNING: Unusually small elapsed time: %u cycles\n", elapsed);
    qemu_printf("  Start: %u, End: %u\n", cycle_start, end_count);
  }

  return elapsed;
}

// Legacy function names to maintain compatibility with the benchmark code
void init_dwt_counter(void) { init_hardware_timer(); }

uint32_t qemu_get_tick_count(void) {
  if (!timer_initialized) {
    init_hardware_timer();
  }

  // Memory barriers for consistency
  __asm__ volatile("dmb" ::: "memory");

  // Check if timer is enabled
  if ((*TIMER1_CONTROL & TIMER_CTRL_ENABLE) == 0) {
    // Re-enable
    uint32_t control = *TIMER1_CONTROL & ~TIMER_CTRL_ENABLE;
    *TIMER1_CONTROL = control | TIMER_CTRL_ENABLE;
    __asm__ volatile("dmb" ::: "memory");
  }

  // Read the current timer value
  return *TIMER1_VALUE;
}

// QEMU semihosting file operations based on ARM semihosting spec
int qemu_file_open(const char *filename, const char *mode) {
  int fd;

  // Convert mode string to a value
  int mode_val = 0;
  if (strchr(mode, 'r') && !strchr(mode, '+'))
    mode_val = 0; // read
  if (strchr(mode, 'r') && strchr(mode, '+'))
    mode_val = 2; // read/write
  if (strchr(mode, 'w'))
    mode_val = 4; // write/create
  if (strchr(mode, 'a'))
    mode_val = 8; // append

  // Call semihosting SYS_OPEN
  __asm__ volatile(
      "mov r0, #0x01\n" // SYS_OPEN
      "mov r1, %1\n"    // Address of parameters
      "bkpt #0xAB\n"    // Semihosting breakpoint
      "mov %0, r0\n"    // Get result
      : "=r"(fd)
      : "r"((void *[]){(void *)filename, (void *)(uintptr_t)mode_val,
                       (void *)strlen(filename)})
      : "r0", "r1", "memory");

  return fd;
}

int qemu_file_close(int fd) {
  int result;

  // Call semihosting SYS_CLOSE
  __asm__ volatile("mov r0, #0x02\n" // SYS_CLOSE
                   "mov r1, %1\n"    // Address of parameters
                   "bkpt #0xAB\n"    // Semihosting breakpoint
                   "mov %0, r0\n"    // Get result
                   : "=r"(result)
                   : "r"(&fd)
                   : "r0", "r1", "memory");

  return result;
}

int qemu_file_read(int fd, void *buf, int len) {
  int result;
  struct {
    int fd;
    void *buf;
    int len;
  } params = {fd, buf, len};

  // Call semihosting SYS_READ
  __asm__ volatile("mov r0, #0x06\n" // SYS_READ
                   "mov r1, %1\n"    // Address of parameters
                   "bkpt #0xAB\n"    // Semihosting breakpoint
                   "mov %0, r0\n"    // Get result
                   : "=r"(result)
                   : "r"(&params)
                   : "r0", "r1", "memory");

  return result;
}

int qemu_file_write(int fd, const void *buf, int len) {
  int result;
  struct {
    int fd;
    const void *buf;
    int len;
  } params = {fd, buf, len};

  // Call semihosting SYS_WRITE
  __asm__ volatile("mov r0, #0x05\n" // SYS_WRITE
                   "mov r1, %1\n"    // Address of parameters
                   "bkpt #0xAB\n"    // Semihosting breakpoint
                   "mov %0, r0\n"    // Get result
                   : "=r"(result)
                   : "r"(&params)
                   : "r0", "r1", "memory");

  return result;
}
