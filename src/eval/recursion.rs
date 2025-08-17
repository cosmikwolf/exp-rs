extern crate alloc;
use crate::error::ExprError;
#[cfg(not(test))]
use alloc::format;

// Add recursion depth tracking for evaluating expression functions
#[cfg(not(test))]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

// Define a static atomic counter for recursion depth
// Using atomics avoids need for RefCell and thread_local
pub static RECURSION_DEPTH: AtomicUsize = AtomicUsize::new(0);

// Add a constant for maximum recursion depth
// We're tracking function calls, so we can use a reasonable limit for call depth
// This should allow for legitimate recursive functions but catch infinite recursion
// Note: On embedded systems or with limited stack space, this may need to be lower
#[cfg(test)]
const MAX_RECURSION_DEPTH: usize = 10; // Very low limit for tests to avoid stack overflow
#[cfg(not(test))]
const MAX_RECURSION_DEPTH: usize = 100; // Conservative limit for production

// Add a helper function to check and increment recursion depth
pub fn check_and_increment_recursion_depth() -> Result<(), ExprError> {
    let current = RECURSION_DEPTH.load(Ordering::Relaxed);

    // Safety check: If the counter is abnormally high but not at the limit,
    // it might indicate a leak from a previous test or evaluation
    if current > MAX_RECURSION_DEPTH - 10 && current < MAX_RECURSION_DEPTH {
        // Log a warning in debug builds
        #[cfg(test)]
        eprintln!(
            "WARNING: Unusually high recursion depth detected: {}",
            current
        );
    }

    if current >= MAX_RECURSION_DEPTH {
        // Immediately reset the counter to prevent potential state inconsistency
        // This ensures future evaluations don't start with a maxed-out counter
        RECURSION_DEPTH.store(0, Ordering::Relaxed);

        Err(ExprError::RecursionLimit(format!(
            "Maximum recursion depth of {} exceeded during expression evaluation",
            MAX_RECURSION_DEPTH
        )))
    } else {
        RECURSION_DEPTH.store(current + 1, Ordering::Relaxed);
        Ok(())
    }
}

// Add a helper function to decrement recursion depth
pub fn decrement_recursion_depth() {
    let current = RECURSION_DEPTH.load(Ordering::Relaxed);
    if current > 0 {
        RECURSION_DEPTH.store(current - 1, Ordering::Relaxed);
    }
}

// Get the current recursion depth - exposed for testing
pub fn get_recursion_depth() -> usize {
    RECURSION_DEPTH.load(Ordering::Relaxed)
}

// Reset the recursion depth counter to zero - exposed for testing
pub fn reset_recursion_depth() {
    RECURSION_DEPTH.store(0, Ordering::Relaxed)
}

// Set the recursion depth to a specific value - exposed for testing
pub fn set_max_recursion_depth(depth: usize) -> usize {
    // We can't actually modify the const, but we can expose this for documentation
    // of test expectations
    depth
}
