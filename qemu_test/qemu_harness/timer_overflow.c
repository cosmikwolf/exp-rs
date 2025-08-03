// Timer overflow handling for accurate long-duration measurements
#include <stdint.h>
#include "qemu_test_harness.h"

// Global overflow counter
static volatile uint32_t timer_overflow_count = 0;

// Timer interrupt handler
void Timer1_Handler(void) {
    // Check if this is a timer overflow interrupt
    if (*TIMER1_RIS & 0x1) {
        // Increment overflow counter
        timer_overflow_count++;
        
        // Clear the interrupt
        *TIMER1_INTCLR = 1;
    }
}

// Reset overflow counter
void reset_overflow_counter(void) {
    timer_overflow_count = 0;
}

// Get current overflow count
uint32_t get_overflow_count(void) {
    return timer_overflow_count;
}

// Calculate total elapsed ticks including overflows
uint64_t calculate_total_ticks(uint32_t start_value, uint32_t end_value, 
                               uint32_t start_overflows, uint32_t end_overflows) {
    uint64_t total_ticks = 0;
    
    // Calculate overflows that occurred
    uint32_t overflow_diff = end_overflows - start_overflows;
    
    // Each overflow represents a full 32-bit count
    total_ticks = (uint64_t)overflow_diff * 0x100000000ULL;
    
    // Add the partial count
    if (start_value >= end_value) {
        // Normal case: no wrap during this measurement
        total_ticks += (start_value - end_value);
    } else {
        // Wrapped once more (but interrupt might not have fired yet)
        total_ticks += (0xFFFFFFFF - end_value) + start_value + 1;
    }
    
    return total_ticks;
}