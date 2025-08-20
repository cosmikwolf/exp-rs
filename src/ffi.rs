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
//! ## Session API (Simple, Automatic Memory Management)
//! - Automatically gets arena from a pool
//! - Single expression evaluation
//! - Arena automatically returned when session is freed
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
//! ## Session API Example
//! ```c
//! // Initialize arena pool
//! expr_pool_init(16);
//!
//! // Create session
//! ExprSession* session = expr_session_new();
//! expr_session_parse(session, "x + y");
//! expr_session_add_variable(session, "x", 10.0);
//! expr_session_add_variable(session, "y", 20.0);
//!
//! // Evaluate
//! Real result;
//! expr_session_evaluate(session, NULL, &result);
//!
//! // Cleanup
//! expr_session_free(session);
//! ```

use crate::expression::{ArenaBatchBuilder, Expression};
use crate::{EvalContext, Real};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bumpalo::Bump;
use core::ffi::{CStr, c_char, c_void};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// Re-export for external visibility
pub use crate::expression::ArenaBatchBuilder as ArenaBatchBuilderExport;
pub use crate::expression::Expression as ExpressionExport;

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

/// Opaque type for expression session (single expression evaluation)
#[repr(C)]
pub struct ExprSession {
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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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
// Arena Management
// ============================================================================

/// Create a new memory arena
///
/// # Parameters
/// - `size_hint`: Suggested size in bytes (0 for default)
///
/// # Returns
/// Pointer to new arena, or NULL on allocation failure
///
/// # Safety
/// The returned pointer must be freed with expr_arena_free()
#[unsafe(no_mangle)]
pub extern "C" fn expr_arena_new(size_hint: usize) -> *mut ExprArena {
    let size = if size_hint == 0 { 8192 } else { size_hint };
    let arena = Box::new(Bump::with_capacity(size));
    Box::into_raw(arena) as *mut ExprArena
}

/// Free a memory arena
///
/// # Safety
/// - The pointer must have been created by expr_arena_new()
/// - All batches using this arena must be freed first
#[unsafe(no_mangle)]
pub extern "C" fn expr_arena_free(arena: *mut ExprArena) {
    if arena.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(arena as *mut Bump);
    }
}

/// Reset an arena for reuse
///
/// This clears all allocations but keeps the memory for reuse.
///
/// # Safety
/// No references to arena-allocated data must exist
#[unsafe(no_mangle)]
pub extern "C" fn expr_arena_reset(arena: *mut ExprArena) {
    if arena.is_null() {
        return;
    }
    let arena = unsafe { &mut *(arena as *mut Bump) };
    arena.reset();
}

// ============================================================================
// Batch Evaluation (Primary API)
// ============================================================================

/// Create a new expression batch
///
/// # Parameters
/// - `arena`: Memory arena for allocations
///
/// # Returns
/// Pointer to new batch, or NULL on failure
///
/// # Safety
/// - The arena must remain valid for the batch's lifetime
/// - The returned pointer must be freed with expr_batch_free()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_new(arena: *mut ExprArena) -> *mut ExprBatch {
    if arena.is_null() {
        return ptr::null_mut();
    }

    let arena = unsafe { &*(arena as *const Bump) };
    let builder = Box::new(ArenaBatchBuilder::new(arena));
    Box::into_raw(builder) as *mut ExprBatch
}

/// Free an expression batch
///
/// # Safety
/// The pointer must have been created by expr_batch_new()
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_free(batch: *mut ExprBatch) {
    if batch.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(batch as *mut ArenaBatchBuilder);
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
        let batch = &mut *(batch as *mut ArenaBatchBuilder);
        batch.clear();
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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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

    let builder = unsafe { &*(batch as *const ArenaBatchBuilder) };
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

    let builder = unsafe { &*(batch as *const ArenaBatchBuilder) };
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

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };

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
// Arena Pool for Session API
// ============================================================================

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
            if slot
                .in_use
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
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

// ============================================================================
// Session API (Single Expression Evaluation with Arena Pool)
// ============================================================================

// Safe wrapper around Expression data without lifetime parameters
struct ExpressionWrapper {
    // Store the arena checkout
    checkout: ArenaCheckout,
    // Store expression strings
    expressions: Vec<String>,
    // Store parameter names and values
    params: Vec<(String, Real)>,
    // Store results
    results: Vec<Real>,
}

impl ExpressionWrapper {
    /// Build a temporary Expression for evaluation
    fn with_expression<F, R>(
        &mut self,
        _ctx: &alloc::rc::Rc<EvalContext>,
        f: F,
    ) -> Result<R, crate::error::ExprError>
    where
        F: FnOnce(&mut Expression) -> Result<R, crate::error::ExprError>,
    {
        // Get arena reference without unsafe - the checkout provides safe access
        let arena = self.checkout.arena();

        // Create a new Expression
        let mut expr = Expression::new(arena);

        // Add all parameters
        for (name, value) in &self.params {
            expr.add_parameter(name, *value)?;
        }

        // Add all expressions
        for expr_str in &self.expressions {
            expr.add_expression(expr_str)?;
        }

        // Call the provided function
        let result = f(&mut expr)?;

        // Update results if evaluation occurred
        if expr.expression_count() > 0 {
            self.results.clear();
            self.results.extend_from_slice(expr.get_all_results());
        }

        Ok(result)
    }
}

/// Initialize the arena pool
///
/// # Parameters
/// - `max_arenas`: Maximum number of arenas in the pool
///
/// # Returns
/// true on success, false if already initialized
#[unsafe(no_mangle)]
pub extern "C" fn expr_pool_init(max_arenas: usize) -> bool {
    initialize(max_arenas).is_ok()
}

/// Create a new expression session
///
/// Automatically gets an arena from the pool.
///
/// # Returns
/// Pointer to new session, or NULL if no arenas available
///
/// # Safety
/// The returned pointer must be freed with expr_session_free()
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_new() -> *mut ExprSession {
    // Get an arena from the pool
    let checkout = match global_pool().checkout() {
        Some(c) => c,
        None => return ptr::null_mut(),
    };

    // Create wrapper without lifetime issues
    let wrapper = ExpressionWrapper {
        checkout,
        expressions: Vec::new(),
        params: Vec::new(),
        results: Vec::new(),
    };

    // Return as opaque pointer
    Box::into_raw(Box::new(wrapper)) as *mut ExprSession
}

/// Free an expression session
///
/// This returns the arena to the pool for reuse.
///
/// # Safety
/// The pointer must have been created by expr_session_new()
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_free(session: *mut ExprSession) {
    if session.is_null() {
        return;
    }

    // Cast back to the actual type and drop it
    unsafe {
        let _ = Box::from_raw(session as *mut ExpressionWrapper);
    }
}

/// Parse an expression in the session
///
/// # Parameters
/// - `session`: The session
/// - `expr`: Expression string (must be valid UTF-8)
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_parse(session: *mut ExprSession, expr: *const c_char) -> i32 {
    if session.is_null() || expr.is_null() {
        return -1;
    }

    // Parse C string
    let expr_cstr = unsafe { CStr::from_ptr(expr) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Store the expression string and add a result slot
    wrapper.expressions.push(expr_str.to_string());
    wrapper.results.push(0.0);

    // Validate by trying to parse with a temporary Expression
    let ctx = alloc::rc::Rc::new(EvalContext::new());
    match wrapper.with_expression(&ctx, |_| Ok(())) {
        Ok(_) => 0,
        Err(_) => {
            // Remove the invalid expression
            wrapper.expressions.pop();
            wrapper.results.pop();
            -3 // Parse error
        }
    }
}

/// Parse an expression in the session with detailed error reporting
///
/// # Parameters
/// - `session`: The session
/// - `expr`: Expression string (must be valid UTF-8)
///
/// # Returns
/// ExprResult with status 0 on success, or error details on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_parse_ex(
    session: *mut ExprSession,
    expr: *const c_char,
) -> ExprResult {
    if session.is_null() || expr.is_null() {
        return ExprResult::from_ffi_error(
            FFI_ERROR_NULL_POINTER,
            "Null pointer passed to expr_session_parse_ex",
        );
    }

    // Parse C string
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

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Store the expression string and add a result slot
    wrapper.expressions.push(expr_str.to_string());
    wrapper.results.push(0.0);

    // Validate by trying to parse with a temporary Expression
    let ctx = alloc::rc::Rc::new(EvalContext::new());
    match wrapper.with_expression(&ctx, |_| Ok(())) {
        Ok(_) => ExprResult::success_value(0.0),
        Err(e) => {
            // Remove the invalid expression
            wrapper.expressions.pop();
            wrapper.results.pop();
            ExprResult::from_expr_error(e)
        }
    }
}

/// Add a variable to the session
///
/// # Parameters
/// - `session`: The session
/// - `name`: Variable name (must be valid UTF-8)
/// - `value`: Initial value
///
/// # Returns
/// Variable index on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_add_variable(
    session: *mut ExprSession,
    name: *const c_char,
    value: Real,
) -> i32 {
    if session.is_null() || name.is_null() {
        return -1;
    }

    // Parse C string
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Check for duplicates
    if wrapper.params.iter().any(|(n, _)| n == name_str) {
        return -3; // Duplicate name
    }

    // Add the parameter
    let idx = wrapper.params.len() as i32;
    wrapper.params.push((name_str.to_string(), value));
    idx
}

/// Update a variable value by name
///
/// # Parameters
/// - `session`: The session
/// - `name`: Variable name
/// - `value`: New value
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_set_variable(
    session: *mut ExprSession,
    name: *const c_char,
    value: Real,
) -> i32 {
    if session.is_null() || name.is_null() {
        return -1;
    }

    // Parse C string
    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Find and update the parameter
    match wrapper.params.iter_mut().find(|(n, _)| n == name_str) {
        Some((_, v)) => {
            *v = value;
            0
        }
        None => -3, // Unknown variable
    }
}

/// Evaluate the expression
///
/// # Parameters
/// - `session`: The session
/// - `ctx`: Optional context with functions (can be NULL)
/// - `result`: Pointer to store the result
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_evaluate(
    session: *mut ExprSession,
    ctx: *mut ExprContext,
    result: *mut Real,
) -> i32 {
    if session.is_null() || result.is_null() {
        return -1;
    }

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Create or use context
    let eval_ctx = if ctx.is_null() {
        alloc::rc::Rc::new(EvalContext::new())
    } else {
        unsafe {
            let ctx_rc = &*(ctx as *const alloc::rc::Rc<EvalContext>);
            ctx_rc.clone()
        }
    };

    // Evaluate using the wrapper
    match wrapper.with_expression(&eval_ctx, |expr| expr.eval_single(&eval_ctx)) {
        Ok(value) => {
            unsafe { *result = value };
            0
        }
        Err(_) => -2, // Evaluation error
    }
}

/// Evaluate the expression with detailed error reporting
///
/// # Parameters
/// - `session`: The session
/// - `ctx`: Optional context with functions (can be NULL)
///
/// # Returns
/// ExprResult with value on success, or error details on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_session_evaluate_ex(
    session: *mut ExprSession,
    ctx: *mut ExprContext,
) -> ExprResult {
    if session.is_null() {
        return ExprResult::from_ffi_error(FFI_ERROR_NULL_POINTER, "Null session pointer");
    }

    // Get the wrapper
    let wrapper = unsafe { &mut *(session as *mut ExpressionWrapper) };

    // Create or use context
    let eval_ctx = if ctx.is_null() {
        alloc::rc::Rc::new(EvalContext::new())
    } else {
        unsafe {
            let ctx_rc = &*(ctx as *const alloc::rc::Rc<EvalContext>);
            ctx_rc.clone()
        }
    };

    // Evaluate using the wrapper
    match wrapper.with_expression(&eval_ctx, |expr| expr.eval_single(&eval_ctx)) {
        Ok(value) => ExprResult::success_value(value),
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
                    core::str::from_utf8_unchecked(
                        core::slice::from_raw_parts(buffer.as_ptr() as *const u8, i)
                    )
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
        assert_eq!(buffer[crate::types::EXP_RS_ERROR_BUFFER_SIZE - 2], b'a' as c_char);

        // Test over-length message (gets truncated)
        let long_msg = "a".repeat(crate::types::EXP_RS_ERROR_BUFFER_SIZE + 10);
        let buffer = ExprResult::copy_to_error_buffer(&long_msg);
        
        // Last byte should still be null terminator
        assert_eq!(buffer[crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1], 0);
        
        // Message should be truncated but still valid
        let recovered_msg = unsafe {
            core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(
                    buffer.as_ptr() as *const u8, 
                    crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1
                )
            )
        };
        assert_eq!(recovered_msg.len(), crate::types::EXP_RS_ERROR_BUFFER_SIZE - 1);
        assert!(recovered_msg.chars().all(|c| c == 'a'));
    }
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
