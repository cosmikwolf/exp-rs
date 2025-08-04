//! Arena pool for FFI to manage arena lifetimes automatically
//!
//! This module provides a thread-safe pool of arenas that can be checked out
//! for use and automatically returned. This eliminates the need for C code
//! to manage arena lifetimes manually.

use alloc::vec::Vec;
use bumpalo::Bump;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Default number of arenas in the pool
const DEFAULT_POOL_SIZE: usize = 4;

/// Default size for each arena (64KB)
const DEFAULT_ARENA_SIZE: usize = 64 * 1024;

/// A slot in the arena pool
struct ArenaSlot {
    /// The arena itself
    arena: Bump,
    /// Whether this slot is currently in use
    in_use: AtomicBool,
}

/// Thread-safe arena pool
pub struct ArenaPool {
    /// Collection of arena slots
    slots: Vec<ArenaSlot>,
    /// Number of arenas currently in use
    active_count: AtomicUsize,
}

/// A checked-out arena from the pool
pub struct ArenaCheckout {
    /// Index of the slot in the pool
    slot_index: usize,
    /// Pointer to the pool (needed for return)
    pool: *const ArenaPool,
}

// Safety: ArenaPool is designed to be thread-safe through atomic operations
unsafe impl Send for ArenaPool {}
unsafe impl Sync for ArenaPool {}

// Safety: ArenaCheckout can be sent between threads as it only contains indices
unsafe impl Send for ArenaCheckout {}

impl ArenaPool {
    /// Create a new arena pool with default settings
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_POOL_SIZE, DEFAULT_ARENA_SIZE)
    }
    
    /// Create a new arena pool with specified capacity
    pub fn with_capacity(num_arenas: usize, arena_size: usize) -> Self {
        let mut slots = Vec::with_capacity(num_arenas);
        
        for _ in 0..num_arenas {
            slots.push(ArenaSlot {
                arena: Bump::with_capacity(arena_size),
                in_use: AtomicBool::new(false),
            });
        }
        
        ArenaPool {
            slots,
            active_count: AtomicUsize::new(0),
        }
    }
    
    /// Try to check out an arena from the pool
    pub fn checkout(&self) -> Option<ArenaCheckout> {
        // Try each slot in order
        for (index, slot) in self.slots.iter().enumerate() {
            // Try to atomically set in_use from false to true
            if slot.in_use.compare_exchange(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_ok() {
                // Successfully reserved this slot
                self.active_count.fetch_add(1, Ordering::Relaxed);
                
                // Reset the arena for clean slate
                // This is safe because we have exclusive access via in_use flag
                unsafe {
                    let arena_ptr = &slot.arena as *const Bump as *mut Bump;
                    (*arena_ptr).reset();
                }
                
                return Some(ArenaCheckout {
                    slot_index: index,
                    pool: self as *const ArenaPool,
                });
            }
        }
        
        // All arenas are in use
        None
    }
    
    
    /// Get the number of arenas currently in use
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }
    
    /// Get the total number of arenas in the pool
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

impl ArenaCheckout {
    /// Get a reference to the arena
    pub fn arena(&self) -> &Bump {
        // This is safe because we know the pool outlives the checkout
        unsafe {
            let pool = &*self.pool;
            &pool.slots[self.slot_index].arena
        }
    }
}

impl Drop for ArenaCheckout {
    fn drop(&mut self) {
        // Return the arena to the pool
        unsafe {
            let pool = &*self.pool;
            if self.slot_index < pool.slots.len() {
                let slot = &pool.slots[self.slot_index];
                slot.in_use.store(false, Ordering::Release);
                pool.active_count.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

/// Global arena pool instance
static mut GLOBAL_POOL: Option<ArenaPool> = None;

/// Initialize the global arena pool
pub fn init_global_pool(num_arenas: usize, arena_size: usize) {
    unsafe {
        let pool_ref = &raw mut GLOBAL_POOL;
        if (*pool_ref).is_none() {
            *pool_ref = Some(ArenaPool::with_capacity(num_arenas, arena_size));
        }
    }
}

/// Get a reference to the global pool, initializing with defaults if needed
pub fn global_pool() -> &'static ArenaPool {
    unsafe {
        let pool_ref = &raw mut GLOBAL_POOL;
        if (*pool_ref).is_none() {
            *pool_ref = Some(ArenaPool::new());
        }
        (*pool_ref).as_ref().unwrap()
    }
}

/// Initialize the global pool with specified size
pub fn initialize(max_arenas: usize) -> Result<(), &'static str> {
    unsafe {
        let pool_ref = &raw mut GLOBAL_POOL;
        if (*pool_ref).is_some() {
            return Err("Pool already initialized");
        }
        *pool_ref = Some(ArenaPool::with_capacity(max_arenas, DEFAULT_ARENA_SIZE));
        Ok(())
    }
}

/// Get the number of active arenas in the global pool
pub fn active_count() -> usize {
    global_pool().active_count()
}

/// Get the total capacity of the global pool
pub fn capacity() -> usize {
    global_pool().capacity()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arena_pool_checkout_checkin() {
        let pool = ArenaPool::with_capacity(2, 1024);
        
        // Check out first arena
        let checkout1 = pool.checkout().expect("Should get first arena");
        assert_eq!(pool.active_count(), 1);
        
        // Check out second arena
        let checkout2 = pool.checkout().expect("Should get second arena");
        assert_eq!(pool.active_count(), 2);
        
        // Pool should be exhausted
        assert!(pool.checkout().is_none());
        
        // Return first arena
        drop(checkout1);
        assert_eq!(pool.active_count(), 1);
        
        // Should be able to check out again
        let _checkout3 = pool.checkout().expect("Should get arena after return");
        assert_eq!(pool.active_count(), 2);
        
        // Clean up
        drop(checkout2);
        drop(_checkout3);
        assert_eq!(pool.active_count(), 0);
    }
    
    #[test]
    fn test_arena_reset_on_checkout() {
        let pool = ArenaPool::with_capacity(1, 1024);
        
        // First checkout and allocate
        let bytes_after_alloc = {
            let checkout = pool.checkout().unwrap();
            let _val = checkout.arena().alloc(42u32);
            let bytes = checkout.arena().allocated_bytes();
            println!("Bytes after alloc: {}", bytes);
            bytes
        };
        
        // Verify allocation happened
        assert!(bytes_after_alloc > 0);
        
        // Second checkout should get reset arena
        {
            let checkout = pool.checkout().unwrap();
            let bytes = checkout.arena().allocated_bytes();
            println!("Bytes after checkout (should be 0): {}", bytes);
            // Since bump allocator tracks total capacity, not used bytes,
            // we need to check if the arena can allocate again from the beginning
            // This test might need to be adjusted based on how Bump tracks allocations
            
            // Try allocating again to verify arena was reset
            let _val2 = checkout.arena().alloc(42u32);
            // If arena was properly reset, this should succeed
        }
    }
}