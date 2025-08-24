//! Foreign Function Interface (FFI) for C/C++ interoperability
//!
//! This module provides a simplified C API for expression evaluation with arena-based memory management.
//!
//! # Overview
//!
//! The exp-rs FFI provides two main APIs:
//!
//! ## Batch API (Advanced, Manual Memory Management)
//! - Create an arena for memory allocation
//! - Create a batch builder with the arena
//! - Add multiple expressions and parameters
//! - Evaluate all expressions at once
//! - Manually manage arena lifetime
//!
//!
//! ## Function Support
//!
//! The FFI supports two types of functions:
//!
//! ### Native Functions
//! - Implemented in C and passed as function pointers
//! - Registered with `expr_context_add_function()`
//! - Example: `sin`, `cos`, `sqrt` implementations
//!
//! ### Expression Functions
//! - Mathematical expressions that can call other functions
//! - Defined as strings and parsed when registered
//! - Registered with `expr_context_add_expression_function()`
//! - Can be removed with `expr_context_remove_expression_function()`
//! - Example: `distance(x1,y1,x2,y2) = sqrt((x2-x1)^2 + (y2-y1)^2)`
//!
//! # Example Usage
//!
//! ## Batch API Example
//! ```c
//! // Create context with functions
//! ExprContext* ctx = expr_context_new();
//! expr_context_add_function(ctx, "sin", 1, native_sin);
//!
//! // Add expression functions (mathematical expressions that can call other functions)
//! expr_context_add_expression_function(ctx, "distance", "x1,y1,x2,y2",
//!                                      "sqrt((x2-x1)^2 + (y2-y1)^2)");
//! expr_context_add_expression_function(ctx, "avg", "a,b", "(a+b)/2");
//!
//! // Create arena and batch
//! ExprArena* arena = expr_arena_new(8192);
//! ExprBatch* batch = expr_batch_new(arena);
//!
//! // Add expressions and parameters
//! expr_batch_add_expression(batch, "x + sin(y)");
//! expr_batch_add_expression(batch, "distance(0, 0, x, y)");
//! expr_batch_add_variable(batch, "x", 1.0);
//! expr_batch_add_variable(batch, "y", 3.14159);
//!
//! // Evaluate
//! expr_batch_evaluate(batch, ctx);
//! Real result1 = expr_batch_get_result(batch, 0);
//! Real result2 = expr_batch_get_result(batch, 1);
//!
//! // Remove expression functions when no longer needed
//! expr_context_remove_expression_function(ctx, "avg");
//!
//! // Cleanup
//! expr_batch_free(batch);
//! expr_arena_free(arena);
//! expr_context_free(ctx);
//! ```
//!

use crate::expression::ArenaBatchBuilder;
use crate::{EvalContext, Real};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use bumpalo::Bump;
use core::ffi::{CStr, c_char, c_void};
use core::ptr;

// Re-export for external visibility
pub use crate::expression::ArenaBatchBuilder as ArenaBatchBuilderExport;

// Magic numbers to detect valid vs freed batches
// Using 32-bit values for compatibility with 32-bit systems
const BATCH_MAGIC: usize = 0x7A9F4E82; // Random 32-bit value for valid batch
const BATCH_FREED: usize = 0x9C2E8B7D; // Random 32-bit value for freed batch

// Internal wrapper that owns both the arena and the batch
struct BatchWithArena {
    magic: usize,                           // Magic number for validation
    arena: *mut Bump,                       // Raw pointer to the arena we leaked
    batch: *mut ArenaBatchBuilder<'static>, // Raw pointer to the batch
}

impl Drop for BatchWithArena {
    fn drop(&mut self) {
        // Mark as freed to detect double-free
        self.magic = BATCH_FREED;

        // Drop the batch first (it has references into the arena)
        if !self.batch.is_null() {
            unsafe {
                let _ = Box::from_raw(self.batch);
            }
            self.batch = ptr::null_mut();
        }
        // Then drop the arena
        if !self.arena.is_null() {
            unsafe {
                let _ = Box::from_raw(self.arena);
            }
            self.arena = ptr::null_mut();
        }
    }
}

// ============================================================================
// Global Allocator for no_std ARM
// ============================================================================

mod allocator {
    use core::alloc::{GlobalAlloc, Layout};

    // Choose between standard and custom allocator based on feature
    #[cfg(feature = "custom_cbindgen_alloc")]
    struct CustomAllocator;

    #[cfg(not(feature = "custom_cbindgen_alloc"))]
    struct StandardAllocator;

    // External function declarations
    #[cfg(feature = "custom_cbindgen_alloc")]
    unsafe extern "C" {
        // Use custom allocation functions provided by the user
        fn exp_rs_malloc(size: usize) -> *mut core::ffi::c_void;
        fn exp_rs_free(ptr: *mut core::ffi::c_void);
    }

    #[cfg(not(feature = "custom_cbindgen_alloc"))]
    unsafe extern "C" {
        // Use standard C malloc/free
        fn malloc(size: usize) -> *mut core::ffi::c_void;
        fn free(ptr: *mut core::ffi::c_void);
    }

    // Implementation for custom allocator (using exp_rs_malloc/exp_rs_free)
    #[cfg(feature = "custom_cbindgen_alloc")]
    unsafe impl GlobalAlloc for CustomAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            let align = layout.align();

            if size == 0 {
                return align as *mut u8;
            }

            // For alignment requirements greater than what exp_rs_malloc guarantees (8 bytes),
            // we need to allocate extra space and manually align
            if align > 8 {
                // Allocate extra space for alignment
                let total_size = size + align;
                // SAFETY: exp_rs_malloc is a custom allocator function provided by the C side
                let ptr = (unsafe { exp_rs_malloc(total_size) }) as *mut u8;
                if ptr.is_null() {
                    return ptr;
                }

                // Calculate aligned address
                let addr = ptr as usize;
                let aligned_addr = (addr + align - 1) & !(align - 1);
                aligned_addr as *mut u8
            } else {
                // exp_rs_malloc already guarantees 8-byte alignment
                // SAFETY: exp_rs_malloc is a custom allocator function provided by the C side
                (unsafe { exp_rs_malloc(size) }) as *mut u8
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            if !ptr.is_null() {
                // For over-aligned allocations, we can't easily find the original pointer
                // This is a limitation - for now just free the given pointer
                // In production code, you'd want to store the original pointer somewhere
                // SAFETY: exp_rs_free is a custom deallocator function provided by the C side
                unsafe { exp_rs_free(ptr as *mut core::ffi::c_void) };
            }
        }
    }

    // Implementation for standard allocator (using malloc/free)
    #[cfg(not(feature = "custom_cbindgen_alloc"))]
    unsafe impl GlobalAlloc for StandardAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            let align = layout.align();

            if size == 0 {
                return align as *mut u8;
            }

            // Standard malloc typically provides 8-byte alignment on 32-bit systems
            // For higher alignment requirements, we need to handle it manually
            if align > 8 {
                // Allocate extra space for alignment
                let total_size = size + align;
                let ptr = (unsafe { malloc(total_size) }) as *mut u8;
                if ptr.is_null() {
                    return ptr;
                }

                // Calculate aligned address
                let addr = ptr as usize;
                let aligned_addr = (addr + align - 1) & !(align - 1);
                aligned_addr as *mut u8
            } else {
                // Standard malloc should provide adequate alignment
                (unsafe { malloc(size) }) as *mut u8
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            if !ptr.is_null() {
                // For over-aligned allocations, we can't easily find the original pointer
                // This is a limitation - for now just free the given pointer
                unsafe {
                    free(ptr as *mut core::ffi::c_void);
                }
            }
        }
    }

    // Choose the appropriate allocator
    #[cfg(feature = "custom_cbindgen_alloc")]
    #[global_allocator]
    static ALLOCATOR: CustomAllocator = CustomAllocator;

    #[cfg(not(feature = "custom_cbindgen_alloc"))]
    #[global_allocator]
    static ALLOCATOR: StandardAllocator = StandardAllocator;
}

// ============================================================================
// Panic Handler Support
// ============================================================================

/// Global panic flag pointer - set by C code
#[allow(dead_code)]
static mut EXP_RS_PANIC_FLAG: *mut i32 = ptr::null_mut();

/// Global log function pointer - set by C code
#[allow(dead_code)]
static mut EXP_RS_LOG_FUNCTION: *const c_void = ptr::null();

/// Type for the logging function
#[allow(dead_code)]
type LogFunctionType = unsafe extern "C" fn(*const u8, usize);

/// Default panic message
#[allow(dead_code)]
static PANIC_DEFAULT_MSG: &[u8] = b"Rust panic occurred\0";

/// Register a panic handler
///
/// # Parameters
/// - `flag_ptr`: Pointer to an integer that will be set to 1 on panic
/// - `log_func`: Optional logging function pointer (can be NULL)
///
/// # Safety
/// The provided pointers must remain valid for the lifetime of the program
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exp_rs_register_panic_handler(
    flag_ptr: *mut i32,
    log_func: *const c_void,
) {
    unsafe {
        EXP_RS_PANIC_FLAG = flag_ptr;
        EXP_RS_LOG_FUNCTION = log_func;
    }
}

// ============================================================================
// Error Handling
// ============================================================================

/// Result structure for FFI operations
#[repr(C)]
pub struct ExprResult {
    /// Error code: 0 for success, positive for ExprError, negative for FFI errors
    status: i32,
    /// Result value (valid only if status == 0)
    value: Real,
    /// Result index (for functions that return an index)
    index: i32,
    /// Error message buffer (empty string on success, no freeing needed)
    error: [c_char; crate::types::EXP_RS_ERROR_BUFFER_SIZE],
}

impl ExprResult {
    /// Helper function to copy a string to the error buffer
    fn copy_to_error_buffer(msg: &str) -> [c_char; crate::types::EXP_RS_ERROR_BUFFER_SIZE] {
        let mut buffer = [0; crate::types::EXP_RS_ERROR_BUFFER_SIZE];
        let bytes = msg.as_bytes();
        let copy_len = core::cmp::min(bytes.len(), crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1);

        for i in 0..copy_len {
            buffer[i] = bytes[i] as c_char;
        }
        buffer[copy_len] = 0; // Null terminator
        buffer
    }
    /// Create a success result with a value
    fn success_value(value: Real) -> Self {
        ExprResult {
            status: 0,
            value,
            index: 0,
            error: [0; crate::types::EXP_RS_ERROR_BUFFER_SIZE],
        }
    }

    /// Create a success result with an index
    fn success_index(index: usize) -> Self {
        ExprResult {
            status: 0,
            value: 0.0,
            index: index as i32,
            error: [0; crate::types::EXP_RS_ERROR_BUFFER_SIZE],
        }
    }

    /// Create an error result from an ExprError
    fn from_expr_error(err: crate::error::ExprError) -> Self {
        let error_code = err.error_code();
        let error_msg = err.to_string(); // Use Display trait

        ExprResult {
            status: error_code,
            value: Real::NAN,
            index: -1,
            error: Self::copy_to_error_buffer(&error_msg),
        }
    }

    /// Create an error result for FFI-specific errors
    fn from_ffi_error(code: i32, msg: &str) -> Self {
        ExprResult {
            status: code,
            value: Real::NAN,
            index: -1,
            error: Self::copy_to_error_buffer(msg),
        }
    }
}

/// FFI error codes (negative to distinguish from ExprError codes)
pub const FFI_ERROR_NULL_POINTER: i32 = -1;
pub const FFI_ERROR_INVALID_UTF8: i32 = -2;
pub const FFI_ERROR_NO_ARENA_AVAILABLE: i32 = -3;
pub const FFI_ERROR_CANNOT_GET_MUTABLE_ACCESS: i32 = -4;
pub const FFI_ERROR_INVALID_POINTER: i32 = -5;

// ============================================================================
// Opaque Types with Better Names
// ============================================================================

/// Opaque type for evaluation context
#[repr(C)]
pub struct ExprContext {
    _private: [u8; 0],
}

/// Opaque type for expression batch
#[repr(C)]
pub struct ExprBatch {
    _private: [u8; 0],
}

/// Opaque type for memory arena
#[repr(C)]
pub struct ExprArena {
    _private: [u8; 0],
}

// ============================================================================
// Native Function Support
// ============================================================================

/// Native function signature
pub type NativeFunc = extern "C" fn(args: *const Real, n_args: usize) -> Real;

// ============================================================================
// Context Management
// ============================================================================

/// Create a new evaluation context
///
/// The context holds function definitions and can be reused across evaluations.
///
/// # Returns
/// Pointer to new context, or NULL on allocation failure
///
/// # Safety
/// The returned pointer must be freed with expr_context_free()
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_new() -> *mut ExprContext {
    let ctx = EvalContext::new();
    let ctx_rc = alloc::rc::Rc::new(ctx);
    let ctx = Box::new(ctx_rc);
    Box::into_raw(ctx) as *mut ExprContext
}

/// Create a new evaluation context without any pre-registered functions
///
/// This creates a context with no built-in functions or constants.
/// Note that basic operators (+, -, *, /, %, <, >, <=, >=, ==, !=) are still
/// available as they are handled by the parser, not the function registry.
///
/// # Returns
/// Pointer to new empty context, or NULL on allocation failure
///
/// # Safety
/// The returned pointer must be freed with expr_context_free()
///
/// # Example
/// ```c
/// ExprContext* ctx = expr_context_new_empty();
/// // Must register all functions manually
/// expr_context_add_function(ctx, "+", 2, add_func);
/// expr_context_add_function(ctx, "*", 2, mul_func);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_new_empty() -> *mut ExprContext {
    let ctx = EvalContext::empty();
    let ctx_rc = alloc::rc::Rc::new(ctx);
    let ctx = Box::new(ctx_rc);
    Box::into_raw(ctx) as *mut ExprContext
}

/// Free an evaluation context
///
/// # Safety
/// - The pointer must have been created by expr_context_new()
/// - The pointer must not be used after calling this function
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_free(ctx: *mut ExprContext) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ctx as *mut alloc::rc::Rc<EvalContext>);
    }
}

/// Get the count of native functions in a context
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_native_function_count(ctx: *const ExprContext) -> usize {
    if ctx.is_null() {
        return 0;
    }

    unsafe {
        let ctx = &*(ctx as *const alloc::rc::Rc<EvalContext>);
        ctx.list_native_functions().len()
    }
}

/// Get the count of expression functions in a context
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_expression_function_count(ctx: *const ExprContext) -> usize {
    if ctx.is_null() {
        return 0;
    }

    unsafe {
        let ctx = &*(ctx as *const alloc::rc::Rc<EvalContext>);
        ctx.list_expression_functions().len()
    }
}

/// Get a native function name by index
/// Returns the length of the name, or 0 if index is out of bounds
/// If buffer is NULL, just returns the length needed
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_get_native_function_name(
    ctx: *const ExprContext,
    index: usize,
    buffer: *mut u8,
    buffer_size: usize,
) -> usize {
    if ctx.is_null() {
        return 0;
    }

    unsafe {
        let ctx = &*(ctx as *const alloc::rc::Rc<EvalContext>);
        let functions = ctx.list_native_functions();

        if index >= functions.len() {
            return 0;
        }

        let name = &functions[index];
        let name_bytes = name.as_bytes();

        if buffer.is_null() {
            return name_bytes.len();
        }

        let copy_len = core::cmp::min(name_bytes.len(), buffer_size);
        core::ptr::copy_nonoverlapping(name_bytes.as_ptr(), buffer, copy_len);

        name_bytes.len()
    }
}

/// Get an expression function name by index
/// Returns the length of the name, or 0 if index is out of bounds
/// If buffer is NULL, just returns the length needed
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_get_expression_function_name(
    ctx: *const ExprContext,
    index: usize,
    buffer: *mut u8,
    buffer_size: usize,
) -> usize {
    if ctx.is_null() {
        return 0;
    }

    unsafe {
        let ctx = &*(ctx as *const alloc::rc::Rc<EvalContext>);
        let functions = ctx.list_expression_functions();

        if index >= functions.len() {
            return 0;
        }

        let name = &functions[index];
        let name_bytes = name.as_bytes();

        if buffer.is_null() {
            return name_bytes.len();
        }

        let copy_len = core::cmp::min(name_bytes.len(), buffer_size);
        core::ptr::copy_nonoverlapping(name_bytes.as_ptr(), buffer, copy_len);

        name_bytes.len()
    }
}

/// Add a native function to the context
///
/// # Parameters
/// - `ctx`: The context
/// - `name`: Function name (must be valid UTF-8)
/// - `arity`: Number of arguments the function expects
/// - `func`: Function pointer
///
/// # Returns
/// 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_add_function(
    ctx: *mut ExprContext,
    name: *const c_char,
    arity: usize,
    func: NativeFunc,
) -> i32 {
    if ctx.is_null() || name.is_null() {
        return -1;
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Create a wrapper that calls the C function
    let implementation = move |args: &[Real]| -> Real {
        if args.len() != arity {
            return Real::NAN;
        }
        func(args.as_ptr(), args.len())
    };

    // Get mutable access to register the function
    match alloc::rc::Rc::get_mut(ctx_handle) {
        Some(ctx_mut) => {
            match ctx_mut.register_native_function(name_str, arity, implementation) {
                Ok(_) => 0,
                Err(_) => -3, // Registration failed
            }
        }
        None => -4, // Cannot get mutable access
    }
}

/// Add an expression function to the context
///
/// Expression functions are mathematical expressions that can call other functions.
/// They are parsed and expanded when used.
///
/// # Parameters
/// - `ctx`: The context
/// - `name`: Function name (must be valid UTF-8)
/// - `params`: Comma-separated parameter names (e.g., "x,y,z")
/// - `expression`: The expression string defining the function
///
/// # Returns
/// 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_add_expression_function(
    ctx: *mut ExprContext,
    name: *const c_char,
    params: *const c_char,
    expression: *const c_char,
) -> i32 {
    if ctx.is_null() || name.is_null() || params.is_null() || expression.is_null() {
        return -1;
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    // Parse function name
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Parse parameters
    let params_cstr = unsafe { CStr::from_ptr(params) };
    let params_str = match params_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Parse expression
    let expr_cstr = unsafe { CStr::from_ptr(expression) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Split parameters by comma
    let param_vec: Vec<&str> = if params_str.is_empty() {
        Vec::new()
    } else {
        params_str.split(',').map(|s| s.trim()).collect()
    };

    // Get mutable access to register the function
    match alloc::rc::Rc::get_mut(ctx_handle) {
        Some(ctx_mut) => {
            // Use validated registration to catch syntax errors during registration
            match ctx_mut
                .register_expression_function_validated(name_str, &param_vec, expr_str, false)
            {
                Ok(report) => {
                    if report.syntax_valid {
                        0 // Success
                    } else {
                        -3 // Syntax validation failed
                    }
                }
                Err(_) => -3, // Registration failed
            }
        }
        None => -4, // Cannot get mutable access
    }
}

/// Remove an expression function from the context
///
/// # Parameters
/// - `ctx`: The context
/// - `name`: Function name to remove
///
/// # Returns
/// - 1 if the function was removed
/// - 0 if the function didn't exist
/// - negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_context_remove_expression_function(
    ctx: *mut ExprContext,
    name: *const c_char,
) -> i32 {
    if ctx.is_null() || name.is_null() {
        return -1;
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Get mutable access to unregister the function
    match alloc::rc::Rc::get_mut(ctx_handle) {
        Some(ctx_mut) => {
            match ctx_mut.unregister_expression_function(name_str) {
                Ok(was_removed) => {
                    if was_removed {
                        1
                    } else {
                        0
                    }
                }
                Err(_) => -3, // Error (e.g., name too long)
            }
        }
        None => -4, // Cannot get mutable access
    }
}

/// Add an expression function to a batch
///
/// Expression functions are mathematical expressions that can call other functions.
/// They are specific to this batch and take precedence over context functions.
///
/// # Parameters
/// - `batch`: The batch
/// - `name`: Function name (must be valid UTF-8)
/// - `params`: Comma-separated parameter names (e.g., "x,y,z")
/// - `expression`: The expression string defining the function
///
/// # Returns
/// 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_add_expression_function(
    batch: *mut ExprBatch,
    name: *const c_char,
    params: *const c_char,
    expression: *const c_char,
) -> i32 {
    if batch.is_null() || name.is_null() || params.is_null() || expression.is_null() {
        return -1;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    // Parse strings
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    let params_cstr = unsafe { CStr::from_ptr(params) };
    let params_str = match params_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    let expr_cstr = unsafe { CStr::from_ptr(expression) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Split parameters by comma
    let param_vec: Vec<&str> = if params_str.is_empty() {
        Vec::new()
    } else {
        params_str.split(',').map(|s| s.trim()).collect()
    };

    // Register function
    match builder.register_expression_function(name_str, &param_vec, expr_str) {
        Ok(_) => 0,
        Err(_) => -3, // Registration failed
    }
}

/// Remove an expression function from a batch
///
/// # Parameters
/// - `batch`: The batch
/// - `name`: Function name to remove
///
/// # Returns
/// - 1 if the function was removed
/// - 0 if the function didn't exist
/// - negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_remove_expression_function(
    batch: *mut ExprBatch,
    name: *const c_char,
) -> i32 {
    if batch.is_null() || name.is_null() {
        return -1;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    match builder.unregister_expression_function(name_str) {
        Ok(was_removed) => {
            if was_removed {
                1
            } else {
                0
            }
        }
        Err(_) => -3, // Error
    }
}

// ============================================================================
// Arena Management - DEPRECATED (arena is now managed internally by batch)
// ============================================================================

// These functions are no longer needed as the batch now manages its own arena.
// They are kept here commented out for reference.

// /// Create a new memory arena
// ///
// /// # Parameters
// /// - `size_hint`: Suggested size in bytes (0 for default)
// ///
// /// # Returns
// /// Pointer to new arena, or NULL on allocation failure
// ///
// /// # Safety
// /// The returned pointer must be freed with expr_arena_free()
// #[unsafe(no_mangle)]
// pub extern "C" fn expr_arena_new(size_hint: usize) -> *mut ExprArena {
//     let size = if size_hint == 0 { 8192 } else { size_hint };
//     let arena = Box::new(Bump::with_capacity(size));
//     Box::into_raw(arena) as *mut ExprArena
// }

// /// Free a memory arena
// ///
// /// # Safety
// /// - The pointer must have been created by expr_arena_new()
// /// - All batches using this arena must be freed first
// #[unsafe(no_mangle)]
// pub extern "C" fn expr_arena_free(arena: *mut ExprArena) {
//     if arena.is_null() {
//         return;
//     }
//     unsafe {
//         let _ = Box::from_raw(arena as *mut Bump);
//     }
// }

// /// Reset an arena for reuse
// ///
// /// This clears all allocations but keeps the memory for reuse.
// ///
// /// # Safety
// /// No references to arena-allocated data must exist
// #[unsafe(no_mangle)]
// pub extern "C" fn expr_arena_reset(arena: *mut ExprArena) {
//     if arena.is_null() {
//         return;
//     }
//     let arena = unsafe { &mut *(arena as *mut Bump) };
//     arena.reset();
// }

// ============================================================================
// Batch Evaluation (Primary API)
// ============================================================================

/// Create a new expression batch with its own arena
///
/// This creates both an arena and a batch in a single allocation.
/// The arena is automatically sized based on the size_hint parameter.
///
/// # Parameters
/// - `size_hint`: Suggested arena size in bytes (0 for default of 8KB)
///
/// # Returns
/// Pointer to new batch, or NULL on failure
///
/// # Safety
/// - The returned pointer must be freed with expr_batch_free()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_new(size_hint: usize) -> *mut ExprBatch {
    // Use default size if 0 is passed
    let arena_size = if size_hint == 0 { 8192 } else { size_hint };

    // Create the arena and leak it to get a 'static reference
    let arena = Box::new(Bump::with_capacity(arena_size));
    let arena_ptr = Box::into_raw(arena);
    let arena_ref: &'static Bump = unsafe { &*arena_ptr };

    // Create the batch with the leaked arena reference
    let batch = Box::new(ArenaBatchBuilder::new(arena_ref));
    let batch_ptr = Box::into_raw(batch);

    // Create the wrapper that tracks both pointers for cleanup
    let wrapper = Box::new(BatchWithArena {
        magic: BATCH_MAGIC,
        arena: arena_ptr,
        batch: batch_ptr,
    });

    Box::into_raw(wrapper) as *mut ExprBatch
}

/// Check if a batch pointer is valid (not freed or corrupted)
///
/// # Parameters
/// - `batch`: The batch pointer to check
///
/// # Returns
/// - ExprResult with status 0 and value 1.0 if the batch is valid
/// - ExprResult with error status and message describing the issue if invalid
///
/// # Safety
/// The pointer should have been created by expr_batch_new()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_is_valid(batch: *const ExprBatch) -> ExprResult {
    if batch.is_null() {
        return ExprResult::from_ffi_error(FFI_ERROR_NULL_POINTER, "Batch pointer is NULL");
    }

    unsafe {
        let wrapper = batch as *const BatchWithArena;
        let magic = (*wrapper).magic;

        if magic == BATCH_MAGIC {
            // Valid batch - return success with value 1.0
            ExprResult::success_value(1.0)
        } else if magic == BATCH_FREED {
            // Batch has been freed
            ExprResult::from_ffi_error(
                FFI_ERROR_INVALID_POINTER,
                "Batch has already been freed (double-free detected)",
            )
        } else {
            // Invalid/corrupted pointer
            ExprResult::from_ffi_error(
                FFI_ERROR_INVALID_POINTER,
                &format!("Invalid or corrupted batch pointer (magic: 0x{:x})", magic),
            )
        }
    }
}

/// Free an expression batch and its arena
///
/// This frees both the batch and its associated arena in one operation.
///
/// # Safety
/// The pointer must have been created by expr_batch_new()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_free(batch: *mut ExprBatch) {
    if batch.is_null() {
        return;
    }

    unsafe {
        // Check the magic number to detect double-free
        let wrapper = batch as *mut BatchWithArena;
        let magic = (*wrapper).magic;

        if magic == BATCH_FREED {
            // Already freed - this is a double-free attempt
            // In debug builds, we could panic here. In release, just return safely.
            #[cfg(debug_assertions)]
            panic!("Double-free detected on ExprBatch at {:p}", batch);

            #[cfg(not(debug_assertions))]
            return; // Silently ignore in release mode
        }

        if magic != BATCH_MAGIC {
            // Invalid magic - this pointer wasn't created by expr_batch_new
            // or memory corruption occurred
            #[cfg(debug_assertions)]
            panic!(
                "Invalid ExprBatch pointer at {:p} (magic: 0x{:x})",
                batch, magic
            );

            #[cfg(not(debug_assertions))]
            return; // Silently ignore in release mode
        }

        // Valid batch - proceed with cleanup
        let _ = Box::from_raw(wrapper);
    }
}

/// Clear all expressions, parameters, and results from a batch
///
/// This allows the batch to be reused without recreating it. The arena memory
/// used by previous expressions remains allocated but unused until the arena
/// is reset. This is safer than freeing and recreating the batch.
///
/// # Parameters
/// - `batch`: The batch to clear
///
/// # Returns
/// 0 on success, negative error code on failure
///
/// # Safety
/// The pointer must have been created by expr_batch_new()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_clear(batch: *mut ExprBatch) -> i32 {
    if batch.is_null() {
        return FFI_ERROR_NULL_POINTER;
    }

    unsafe {
        let wrapper = &mut *(batch as *mut BatchWithArena);

        // Validate magic number
        if wrapper.magic != BATCH_MAGIC {
            #[cfg(debug_assertions)]
            panic!(
                "Invalid or freed ExprBatch pointer at {:p} (magic: 0x{:x})",
                batch, wrapper.magic
            );

            #[cfg(not(debug_assertions))]
            return FFI_ERROR_INVALID_POINTER; // Return error in release mode
        }

        (*wrapper.batch).clear();
    }

    0
}

/// Add an expression to the batch
///
/// # Parameters
/// - `batch`: The batch
/// - `expr`: Expression string (must be valid UTF-8)
///
/// # Returns
/// ExprResult with index on success, or error details on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_add_expression(
    batch: *mut ExprBatch,
    expr: *const c_char,
) -> ExprResult {
    if batch.is_null() || expr.is_null() {
        return ExprResult::from_ffi_error(
            FFI_ERROR_NULL_POINTER,
            "Null pointer passed to expr_batch_add_expression",
        );
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    let expr_cstr = unsafe { CStr::from_ptr(expr) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            return ExprResult::from_ffi_error(
                FFI_ERROR_INVALID_UTF8,
                "Invalid UTF-8 in expression string",
            );
        }
    };

    match builder.add_expression(expr_str) {
        Ok(idx) => ExprResult::success_index(idx),
        Err(e) => ExprResult::from_expr_error(e),
    }
}

/// Add a variable to the batch
///
/// # Parameters
/// - `batch`: The batch
/// - `name`: Variable name (must be valid UTF-8)
/// - `value`: Initial value
///
/// # Returns
/// ExprResult with index on success, or error details on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_add_variable(
    batch: *mut ExprBatch,
    name: *const c_char,
    value: Real,
) -> ExprResult {
    if batch.is_null() || name.is_null() {
        return ExprResult::from_ffi_error(
            FFI_ERROR_NULL_POINTER,
            "Null pointer passed to expr_batch_add_variable",
        );
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            return ExprResult::from_ffi_error(
                FFI_ERROR_INVALID_UTF8,
                "Invalid UTF-8 in variable name",
            );
        }
    };

    match builder.add_parameter(name_str, value) {
        Ok(idx) => ExprResult::success_index(idx),
        Err(e) => ExprResult::from_expr_error(e),
    }
}

/// Update a variable value by index
///
/// # Parameters
/// - `batch`: The batch
/// - `index`: Variable index from expr_batch_add_variable()
/// - `value`: New value
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_set_variable(batch: *mut ExprBatch, index: usize, value: Real) -> i32 {
    if batch.is_null() {
        return -1;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    match builder.set_param(index, value) {
        Ok(_) => 0,
        Err(_) => -2, // Invalid index
    }
}

/// Evaluate all expressions in the batch
///
/// # Parameters
/// - `batch`: The batch
/// - `ctx`: Optional context with functions (can be NULL)
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_evaluate(batch: *mut ExprBatch, ctx: *mut ExprContext) -> i32 {
    if batch.is_null() {
        return -1;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    let eval_ctx = if ctx.is_null() {
        alloc::rc::Rc::new(EvalContext::new())
    } else {
        unsafe {
            let ctx_rc = &*(ctx as *const alloc::rc::Rc<EvalContext>);
            ctx_rc.clone()
        }
    };

    match builder.eval(&eval_ctx) {
        Ok(_) => 0,
        Err(_) => -2, // Evaluation error
    }
}

/// Get the result of an expression
///
/// # Parameters
/// - `batch`: The batch
/// - `index`: Expression index from expr_batch_add_expression()
///
/// # Returns
/// Result value, or NaN if index is invalid
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_get_result(batch: *const ExprBatch, index: usize) -> Real {
    if batch.is_null() {
        return Real::NAN;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &*wrapper.batch };
    builder.get_result(index).unwrap_or(Real::NAN)
}

/// Get the high water mark of arena memory usage for a batch
///
/// # Parameters
/// - `batch`: The batch
///
/// # Returns
/// Number of bytes currently allocated in the batch's arena.
/// This represents the maximum memory usage of the arena.
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_arena_bytes(batch: *const ExprBatch) -> usize {
    if batch.is_null() {
        return 0;
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &*wrapper.batch };
    builder.arena_allocated_bytes()
}

/// Evaluate all expressions in the batch with detailed error reporting
///
/// # Parameters
/// - `batch`: The batch
/// - `ctx`: Optional context with functions (can be NULL)
///
/// # Returns
/// ExprResult with status 0 on success, or error details on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_evaluate_ex(
    batch: *mut ExprBatch,
    ctx: *mut ExprContext,
) -> ExprResult {
    if batch.is_null() {
        return ExprResult::from_ffi_error(FFI_ERROR_NULL_POINTER, "Null batch pointer");
    }

    let wrapper = unsafe { &*(batch as *const BatchWithArena) };
    let builder = unsafe { &mut *wrapper.batch };

    let eval_ctx = if ctx.is_null() {
        alloc::rc::Rc::new(EvalContext::new())
    } else {
        unsafe {
            let ctx_rc = &*(ctx as *const alloc::rc::Rc<EvalContext>);
            ctx_rc.clone()
        }
    };

    match builder.eval(&eval_ctx) {
        Ok(_) => ExprResult::success_value(0.0), // No specific value for batch eval
        Err(e) => ExprResult::from_expr_error(e),
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Estimate arena size needed for expressions
///
/// # Parameters
/// - `expression_count`: Number of expressions
/// - `total_expr_length`: Total length of all expression strings
/// - `param_count`: Number of parameters
/// - `estimated_iterations`: Estimated evaluation iterations
///
/// # Returns
/// Recommended arena size in bytes
#[unsafe(no_mangle)]
pub extern "C" fn expr_estimate_arena_size(
    expression_count: usize,
    total_expr_length: usize,
    param_count: usize,
    _estimated_iterations: usize,
) -> usize {
    // Base overhead per expression (AST nodes, etc)
    let expr_overhead = expression_count * 512;

    // String storage
    let string_storage = total_expr_length * 2;

    // Parameter storage
    let param_storage = param_count * 64;

    // Add 50% buffer
    let total = expr_overhead + string_storage + param_storage;
    total + (total / 2)
}

// ============================================================================
// Test-only Panic Trigger
// ============================================================================

/// Force a panic for testing purposes (only available in debug builds)
#[cfg(debug_assertions)]
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_test_trigger_panic() {
    panic!("Test panic triggered from C");
}

// ============================================================================
// Panic Handler Implementation
// ============================================================================

/// Panic handler for no_std environments (ARM targets)
#[cfg(all(not(test), target_arch = "arm"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Try to set the panic flag to let C code know about the panic
    unsafe {
        if !EXP_RS_PANIC_FLAG.is_null() {
            *EXP_RS_PANIC_FLAG = 1;
        }

        // Try to log if we have a logging function
        if !EXP_RS_LOG_FUNCTION.is_null() {
            // Cast the raw pointer to a function pointer and call it
            let log_func: LogFunctionType = core::mem::transmute(EXP_RS_LOG_FUNCTION);

            // Try to extract panic information
            // Note: The .message() method was removed in newer Rust versions
            // We'll use location information which is more stable
            if let Some(location) = info.location() {
                // Create a simple message with file and line info
                let file = location.file();
                let _line = location.line(); // We have line number but can't easily format it in no_std

                // Log the file path first
                log_func(file.as_ptr(), file.len());

                // In a no_std environment, we can't easily format strings with line numbers
                // The C side logger can at least see which file panicked
            } else {
                // Fallback to default message
                log_func(PANIC_DEFAULT_MSG.as_ptr(), PANIC_DEFAULT_MSG.len() - 1);
            }
        }
    }

    // Trigger a fault that the debugger can catch
    #[cfg(target_arch = "arm")]
    loop {
        unsafe {
            // Trigger a HardFault by executing an undefined instruction
            // This allows the debugger to catch the fault and inspect the state
            core::arch::asm!("udf #0");
        }
        // If the fault handler returns, we'll trigger it again
        // This prevents execution from continuing past the panic
    }

    // Fallback for non-ARM architectures
    #[cfg(not(target_arch = "arm"))]
    loop {
        // Busy loop for debugging - debugger can break here
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_buffer_null_termination() {
        use core::ffi::c_char;

        // Test normal message (well within buffer size)
        let short_msg = "Test error message";
        let buffer = ExprResult::copy_to_error_buffer(short_msg);

        // Find the null terminator
        let mut found_null = false;
        for (i, &byte) in buffer.iter().enumerate() {
            if byte == 0 {
                found_null = true;
                // Verify the message is correct up to null terminator
                let recovered_msg = unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        buffer.as_ptr() as *const u8,
                        i,
                    ))
                };
                assert_eq!(recovered_msg, short_msg);
                break;
            }
        }
        assert!(found_null, "Error buffer should be null terminated");

        // Test maximum length message (exactly buffer size - 1)
        let max_msg = "a".repeat(crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1);
        let buffer = ExprResult::copy_to_error_buffer(&max_msg);

        // Last byte should be null terminator
        assert_eq!(buffer[crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1], 0);

        // Second-to-last byte should contain message data
        assert_eq!(
            buffer[crate::types::EXP_RS_ERROR_BUFFER_SIZE - 2],
            b'a' as c_char
        );

        // Test over-length message (gets truncated)
        let long_msg = "a".repeat(crate::types::EXP_RS_ERROR_BUFFER_SIZE + 10);
        let buffer = ExprResult::copy_to_error_buffer(&long_msg);

        // Last byte should still be null terminator
        assert_eq!(buffer[crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1], 0);

        // Message should be truncated but still valid
        let recovered_msg = unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                buffer.as_ptr() as *const u8,
                crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1,
            ))
        };
        assert_eq!(
            recovered_msg.len(),
            crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1
        );
        assert!(recovered_msg.chars().all(|c| c == 'a'));
    }
}
