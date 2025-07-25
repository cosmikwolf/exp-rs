/*!
 * C FFI interface for exp-rs.
 *
 * This module provides a Foreign Function Interface (FFI) that allows calling exp-rs
 * functionality from C code. It exposes a safe, minimal C-compatible API for evaluating
 * expressions and managing evaluation contexts.
 *
 * # Usage from C
 *
 * ```c
 * #include "exp_rs.h"
 *
 * int main() {
 *     // Simple evaluation without context
 *     EvalResult result = exp_rs_eval("2 * (3 + 4)");
 *     if (result.status == 0) {
 *         printf("Result: %f\n", result.value);  // Prints: Result: 14.000000
 *     } else {
 *         printf("Error: %s\n", result.error);
 *         exp_rs_free_error((char*)result.error);
 *     }
 *
 *     // Using a context for variables and functions
 *     ExpContext* ctx = exp_rs_context_new();
 *
 *     // Set variables
 *     exp_rs_context_set_parameter(ctx, "x", 10.0);
 *     exp_rs_context_set_parameter(ctx, "y", 5.0);
 *
 *     // Register a custom function
 *     const char* params[] = {"a", "b"};
 *     exp_rs_context_register_expression_function(
 *         ctx, "add_squared", params, 2, "a*a + b*b"
 *     );
 *
 *     // Evaluate with context
 *     result = exp_rs_context_eval("add_squared(x, y)", ctx);
 *     if (result.status == 0) {
 *         printf("Result: %f\n", result.value);  // Prints: Result: 125.000000
 *     } else {
 *         printf("Error: %s\n", result.error);
 *         exp_rs_free_error((char*)result.error);
 *     }
 *
 *     // Clean up
 *     exp_rs_context_free(ctx);
 *
 *     return 0;
 * }
 * ```
 *
 * See the include/exp_rs.h header file for the complete C API definition.
 */

extern crate alloc;
use crate::context::EvalContext;
use crate::engine::interp;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char};

// Add allocator code for ARM targets
#[cfg(all(not(test), target_arch = "arm"))]
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
                let ptr = exp_rs_malloc(total_size) as *mut u8;
                if ptr.is_null() {
                    return ptr;
                }

                // Calculate aligned address
                let addr = ptr as usize;
                let aligned_addr = (addr + align - 1) & !(align - 1);
                aligned_addr as *mut u8
            } else {
                // exp_rs_malloc already guarantees 8-byte alignment
                exp_rs_malloc(size) as *mut u8
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            if !ptr.is_null() {
                // For over-aligned allocations, we can't easily find the original pointer
                // This is a limitation - for now just free the given pointer
                // In production code, you'd want to store the original pointer somewhere
                exp_rs_free(ptr as *mut core::ffi::c_void);
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
                let ptr = malloc(total_size) as *mut u8;
                if ptr.is_null() {
                    return ptr;
                }

                // Calculate aligned address
                let addr = ptr as usize;
                let aligned_addr = (addr + align - 1) & !(align - 1);
                aligned_addr as *mut u8
            } else {
                // Standard malloc should provide adequate alignment
                malloc(size) as *mut u8
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            if !ptr.is_null() {
                // For over-aligned allocations, we can't easily find the original pointer
                // This is a limitation - for now just free the given pointer
                free(ptr as *mut core::ffi::c_void);
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

// Global flag for detecting panics in C code
#[unsafe(no_mangle)]
#[cfg(not(test))]
pub static mut EXP_RS_PANIC_FLAG: *mut i32 = core::ptr::null_mut();

// C-compatible function pointer type for logging
#[cfg(not(test))]
pub type LogFunctionType = unsafe extern "C" fn(*const core::ffi::c_char);

// Use a raw function pointer instead of Option<T> for C compatibility
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub static mut EXP_RS_LOG_FUNCTION: *const core::ffi::c_void = core::ptr::null();

// Static message for panic with no text
#[cfg(not(test))]
static PANIC_DEFAULT_MSG: &[u8] = b"Rust panic occurred\0";

// Register a panic handler
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exp_rs_register_panic_handler(
    flag_ptr: *mut i32,
    log_func: *const core::ffi::c_void,
) {
    unsafe {
        EXP_RS_PANIC_FLAG = flag_ptr;
        EXP_RS_LOG_FUNCTION = log_func;
    }
}

// Panic handler for no_std - improved version
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

            // Try to get the string directly from the panic information
            if let Some(msg) = info.message().as_str() {
                // msg is a &str, which is already a valid pointer to string data
                // Make sure string has null termination for C
                if msg.as_bytes().last() == Some(&0) {
                    // Already null-terminated, can use directly
                    log_func(msg.as_ptr() as *const c_char);
                } else {
                    // Not null-terminated, use default
                    log_func(PANIC_DEFAULT_MSG.as_ptr() as *const c_char);
                }
            } else {
                // No message or can't get as_str, use default
                log_func(PANIC_DEFAULT_MSG.as_ptr() as *const c_char);
            }
        }
    }

    // Abort rather than hang
    #[cfg(target_arch = "arm")]
    unsafe {
        core::arch::asm!("bkpt #0");
    }

    // If we can't abort, at least don't hang forever
    loop {
        #[cfg(target_arch = "arm")]
        unsafe {
            // Insert a WFI (Wait For Interrupt) instruction to save power
            core::arch::asm!("wfi");
        }
    }
}

use crate::Real;
use alloc::ffi::CString;
use core::ptr;

// Use the appropriate floating point type for EvalResult
/// Result structure returned by evaluation functions.
///
/// This structure returns either a successful result value or an error message.
/// When status is 0, the value field contains the result of the expression evaluation.
/// When status is non-zero, the error field contains a null-terminated string with
/// the error message, which must be freed using exp_rs_free_error.
#[repr(C)]
pub struct EvalResult {
    /// Status code: 0 for success, non-zero for errors
    pub status: i32,

    /// The result value (valid when status is 0)
    pub value: Real,

    /// Error message (valid when status is non-zero, must be freed by caller)
    pub error: *const c_char,
}

/// Frees a string allocated by exp_rs FFI functions.
///
/// This function should be called to free the error message string in an EvalResult
/// when status is non-zero. Not calling this function will result in a memory leak.
///
/// # Parameters
///
/// * `ptr` - Pointer to the string to free. Must be a pointer returned in an EvalResult error field.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// 1. The pointer is valid and was allocated by one of the exp_rs FFI functions
/// 2. The pointer is not used after calling this function
/// 3. The pointer is not freed more than once
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_free_error(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Evaluates a mathematical expression without a context.
///
/// This function evaluates a mathematical expression string and returns the result.
/// Without a context, only built-in functions and constants are available.
///
/// # Parameters
///
/// * `expr` - Null-terminated string containing the expression to evaluate
///
/// # Returns
///
/// An EvalResult structure containing either the result value or an error message.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// 1. The pointer is valid and points to a null-terminated string
/// 2. The string contains valid UTF-8 data
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_eval(expr: *const c_char) -> EvalResult {
    if expr.is_null() {
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr::null(),
        };
    }
    let cstr = unsafe { CStr::from_ptr(expr) };
    let expr_str = match cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };
    match interp(expr_str, None) {
        Ok(val) => EvalResult {
            status: 0,
            value: val,
            error: ptr::null(),
        },
        Err(e) => {
            let msg = CString::new(format!("{}", e)).unwrap();
            let ptr = msg.into_raw();
            EvalResult {
                status: 3,
                value: Real::NAN,
                error: ptr,
            }
        }
    }
}

/// Opaque handle to an evaluation context for C code.
///
/// This is an opaque type that C code can use to reference an EvalContext.
/// C code should only store and pass this pointer, never dereferencing it directly.
#[repr(C)]
pub struct EvalContextOpaque {
    _private: [u8; 0],
}

/// Creates a new evaluation context.
///
/// This function creates a new evaluation context that can be used to store
/// variables, constants, and functions for use in expressions. The context
/// must be freed with exp_rs_context_free when no longer needed.
///
/// # Returns
///
/// A pointer to the new context, or NULL if allocation failed.
///
/// # Safety
///
/// This function is safe to call from C code. The returned pointer must be
/// passed to exp_rs_context_free when no longer needed to avoid memory leaks.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_new() -> *mut EvalContextOpaque {
    let ctx_rc = alloc::rc::Rc::new(EvalContext::new());
    // Enable AST caching by default for FFI contexts
    ctx_rc.enable_ast_cache();
    let ctx = Box::new(ctx_rc);
    Box::into_raw(ctx) as *mut EvalContextOpaque
}

/// Frees an evaluation context previously created by exp_rs_context_new.
///
/// This function releases all resources associated with the given context.
/// After calling this function, the context pointer is no longer valid and
/// should not be used.
///
/// # Parameters
///
/// * `ctx` - Pointer to the context to free, as returned by exp_rs_context_new
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// 1. The pointer was returned by exp_rs_context_new
/// 2. The pointer has not already been freed
/// 3. The pointer is not used after calling this function
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_free(ctx: *mut EvalContextOpaque) {
    if ctx.is_null() {
        return;
    }
    unsafe {
        // Need to drop the Box<Rc<EvalContext>> explicitly
        let _ = Box::from_raw(ctx as *mut alloc::rc::Rc<EvalContext>);
    }
}

/// Register an expression function with the given context.
///
/// This function registers a new function defined by an expression string
/// that can be called in future expression evaluations.
///
/// # Parameters
///
/// * `ctx` - Pointer to the context, as returned by exp_rs_context_new
/// * `name` - The name of the function to register
/// * `params` - Array of parameter names the function will accept
/// * `param_count` - Number of parameters in the array
/// * `expression` - The expression string that defines the function behavior
///
/// # Returns
///
/// An EvalResult structure with:
/// - status=0 on success
/// - non-zero status with an error message on failure
///
/// When status is non-zero, the error message must be freed with exp_rs_free_error.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_register_expression_function(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
    params: *const *const c_char,
    param_count: usize,
    expression: *const c_char,
) -> EvalResult {
    if ctx.is_null() || name.is_null() || expression.is_null() {
        let msg = CString::new("Null pointer provided for required parameter").unwrap();
        let ptr = msg.into_raw();
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr,
        };
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8 in function name: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };

    let expr_cstr = unsafe { CStr::from_ptr(expression) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8 in expression: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 3,
                value: Real::NAN,
                error: ptr,
            };
        }
    };

    let mut param_vec = Vec::new();
    if !params.is_null() {
        for i in 0..param_count {
            let param_ptr = unsafe { *params.add(i) };
            if param_ptr.is_null() {
                let msg =
                    CString::new(format!("Null pointer in parameter list at index {}", i)).unwrap();
                let ptr = msg.into_raw();
                return EvalResult {
                    status: 4,
                    value: Real::NAN,
                    error: ptr,
                };
            }
            let param_cstr = unsafe { CStr::from_ptr(param_ptr) };
            match param_cstr.to_str() {
                Ok(s) => param_vec.push(s.to_string()),
                Err(e) => {
                    let msg = CString::new(format!(
                        "Invalid UTF-8 in parameter name at index {}: {}",
                        i, e
                    ))
                    .unwrap();
                    let ptr = msg.into_raw();
                    return EvalResult {
                        status: 5,
                        value: Real::NAN,
                        error: ptr,
                    };
                }
            }
        }
    }

    let ctx_mut = alloc::rc::Rc::make_mut(ctx_handle);
    match ctx_mut.register_expression_function(
        name_str,
        &param_vec.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        expr_str,
    ) {
        Ok(_) => EvalResult {
            status: 0,
            value: 0.0,
            error: ptr::null(),
        },
        Err(e) => {
            let msg =
                CString::new(format!("Failed to register expression function: {}", e)).unwrap();
            let ptr = msg.into_raw();
            EvalResult {
                status: 6,
                value: Real::NAN,
                error: ptr,
            }
        }
    }
}

/// Unregister an expression function from the given context.
///
/// This function removes an expression function that was previously registered
/// with exp_rs_context_register_expression_function. It only affects the current
/// context and does not modify parent contexts.
///
/// # Warning
///
/// Unregistering a function that is used by other expression functions may cause
/// runtime errors when those expressions are evaluated later. The AST cache is
/// cleared when a function is unregistered to prevent some issues.
///
/// # Parameters
///
/// * `ctx` - Pointer to the context, as returned by exp_rs_context_new
/// * `name` - The name of the expression function to unregister
///
/// # Returns
///
/// An EvalResult structure with:
/// - status=0 and value=1.0 if the function was found and removed
/// - status=0 and value=0.0 if the function was not found in this context
/// - non-zero status with an error message on failure
///
/// When status is non-zero, the error message must be freed with exp_rs_free_error.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_unregister_expression_function(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
) -> EvalResult {
    if ctx.is_null() || name.is_null() {
        let msg = CString::new("Null pointer provided for required parameter").unwrap();
        let ptr = msg.into_raw();
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr,
        };
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8 in function name: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };

    let ctx_mut = alloc::rc::Rc::make_mut(ctx_handle);
    match ctx_mut.unregister_expression_function(name_str) {
        Ok(was_removed) => EvalResult {
            status: 0,
            value: if was_removed { 1.0 } else { 0.0 },
            error: ptr::null(),
        },
        Err(e) => {
            let msg =
                CString::new(format!("Failed to unregister expression function: {}", e)).unwrap();
            let ptr = msg.into_raw();
            EvalResult {
                status: 3,
                value: Real::NAN,
                error: ptr,
            }
        }
    }
}

/// Register a native function with the given context.
///
/// This function registers a Rust function to be invoked from C expressions.
/// The native function will be available for use in expressions evaluated with this context.
///
/// # Parameters
///
/// * `ctx` - Pointer to the context, as returned by exp_rs_context_new
/// * `name` - The name of the function to register
/// * `arity` - Number of parameters the function accepts
/// * `func_ptr` - Function pointer to the implementation (C callback)
///
/// # Returns
///
/// An EvalResult structure with:
/// - status=0 on success
/// - non-zero status with an error message on failure
///
/// When status is non-zero, the error message must be freed with exp_rs_free_error.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_register_native_function(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
    arity: usize,
    func_ptr: unsafe extern "C" fn(*const Real, usize) -> Real,
) -> EvalResult {
    if ctx.is_null() || name.is_null() {
        let msg = CString::new("Null pointer provided for required parameter").unwrap();
        let ptr = msg.into_raw();
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr,
        };
    }

    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8 in function name: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };

    // Create a Rust closure that calls the C function
    let implementation = move |args: &[Real]| -> Real {
        unsafe {
            // Call the C function with a pointer to the arguments
            func_ptr(args.as_ptr(), args.len())
        }
    };

    // Register the native function with the given name and arity
    let ctx_mut = alloc::rc::Rc::make_mut(ctx_handle);
    ctx_mut.register_native_function(name_str, arity, implementation);

    EvalResult {
        status: 0,
        value: 0.0,
        error: ptr::null(),
    }
}

/// Set a parameter value in the context.
///
/// This function adds or updates a variable in the evaluation context that can be
/// referenced in expressions evaluated with this context.
///
/// # Parameters
///
/// * `ctx` - Pointer to the context, as returned by exp_rs_context_new
/// * `name` - The name of the parameter to set
/// * `value` - The value to assign to the parameter
///
/// # Returns
///
/// An EvalResult structure with:
/// - status=0 on success
/// - non-zero status with an error message on failure
///
/// When status is non-zero, the error message must be freed with exp_rs_free_error.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_set_parameter(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
    value: Real,
) -> EvalResult {
    if ctx.is_null() || name.is_null() {
        let msg = CString::new("Null pointer provided for required parameter").unwrap();
        let ptr = msg.into_raw();
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr,
        };
    }
    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8 in parameter name: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };

    let ctx_mut = alloc::rc::Rc::make_mut(ctx_handle);
    ctx_mut.set_parameter(name_str, value);

    EvalResult {
        status: 0,
        value, // Return the value that was set
        error: ptr::null(),
    }
}

/// Evaluates a mathematical expression using the given context.
///
/// This function evaluates a mathematical expression string using the specified context,
/// which can contain variables, constants, and custom functions.
///
/// # Parameters
///
/// * `expr` - Null-terminated string containing the expression to evaluate
/// * `ctx` - Pointer to the context to use, as returned by exp_rs_context_new
///
/// # Returns
///
/// An EvalResult structure containing either the result value or an error message.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// 1. The expression pointer is valid and points to a null-terminated string
/// 2. The string contains valid UTF-8 data
/// 3. The context pointer was returned by exp_rs_context_new and has not been freed
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_eval(
    expr: *const c_char,
    ctx: *mut EvalContextOpaque,
) -> EvalResult {
    if expr.is_null() || ctx.is_null() {
        return EvalResult {
            status: 1,
            value: Real::NAN,
            error: ptr::null(),
        };
    }
    let ctx_handle = unsafe { &*(ctx as *const alloc::rc::Rc<EvalContext>) };
    let cstr = unsafe { CStr::from_ptr(expr) };
    let expr_str = match cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("Invalid UTF-8: {}", e)).unwrap();
            let ptr = msg.into_raw();
            return EvalResult {
                status: 2,
                value: Real::NAN,
                error: ptr,
            };
        }
    };
    let ctx_rc = ctx_handle.clone(); // Just increments refcount
    match interp(expr_str, Some(ctx_rc)) {
        Ok(val) => EvalResult {
            status: 0,
            value: val,
            error: ptr::null(),
        },
        Err(e) => {
            let msg = CString::new(format!("{}", e)).unwrap();
            let ptr = msg.into_raw();
            EvalResult {
                status: 3,
                value: Real::NAN,
                error: ptr,
            }
        }
    }
}

/// Status information for individual batch evaluation results.
///
/// This structure tracks the outcome of each expression evaluation in a batch,
/// allowing detailed error reporting when processing multiple expressions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BatchStatus {
    /// Error code: 0 = success, non-zero = error
    pub code: i32,
    /// Index of the expression that produced this result (0-based)
    pub expr_index: usize,
    /// Index of the batch item that produced this result (0-based)
    pub batch_index: usize,
}

/// Request structure for batch evaluation of multiple expressions.
///
/// This structure allows efficient evaluation of multiple expressions with
/// different parameter values, reusing parsed ASTs and evaluation engines
/// for significant performance improvements.
///
/// # Memory Layout
///
/// - `expressions`: Array of C strings containing the expressions to evaluate
/// - `param_values`: 2D array where param_values[i] points to an array of values for parameter i
/// - `results`: 2D array where results[i] points to an array to store results for expression i
///
/// # Example
///
/// ```c
/// // Evaluate 3 expressions with 2 parameters over 1000 data points
/// const char* exprs[] = {"a + b", "a * sin(b)", "sqrt(a*a + b*b)"};
/// const char* params[] = {"a", "b"};
/// Real a_values[1000] = {...};
/// Real b_values[1000] = {...};
/// Real* param_vals[] = {a_values, b_values};
/// Real results1[1000], results2[1000], results3[1000];
/// Real* results[] = {results1, results2, results3};
///
/// BatchEvalRequest req = {
///     .expressions = exprs,
///     .expression_count = 3,
///     .param_names = params,
///     .param_count = 2,
///     .param_values = param_vals,
///     .batch_size = 1000,
///     .results = results,
///     .allocate_results = false,
///     .stop_on_error = false,
///     .statuses = NULL
/// };
///
/// int status = exp_rs_batch_eval(&req, ctx);
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BatchEvalRequest {
    /// Array of expression strings to evaluate
    pub expressions: *const *const c_char,
    /// Number of expressions in the array
    pub expression_count: usize,

    /// Array of parameter names (shared across all evaluations)
    pub param_names: *const *const c_char,
    /// Number of parameters
    pub param_count: usize,

    /// 2D array of parameter values: param_values[param_idx][batch_idx]
    pub param_values: *const *const Real,
    /// Number of items in each parameter array (batch size)
    pub batch_size: usize,

    /// 2D array for results: results[expr_idx][batch_idx]
    /// Must point to pre-allocated buffers
    pub results: *mut *mut Real,

    /// If true, stop evaluation on first error
    pub stop_on_error: bool,

    /// Optional array for detailed error tracking
    /// Size should be expression_count * batch_size
    /// Can be NULL if detailed error tracking is not needed
    pub statuses: *mut BatchStatus,
}

/// Result structure for batch evaluation when using library allocation.
///
/// This structure is returned when the library allocates the result arrays,
/// providing both the results and allocation metadata.
#[repr(C)]
pub struct BatchEvalResult {
    /// Allocated 2D result array: results[expr_idx][batch_idx]
    pub results: *mut *mut Real,
    /// Number of expressions (rows in results)
    pub expression_count: usize,
    /// Number of batch items (columns in results)
    pub batch_size: usize,
    /// Overall status: 0 = success, non-zero = error
    pub status: i32,
}

/// Evaluates multiple expressions with multiple parameter sets in a batch.
///
/// This function provides high-performance batch evaluation by:
/// - Parsing each expression only once
/// - Reusing a single evaluation engine for all evaluations
/// - Minimizing FFI overhead
///
/// # Parameters
///
/// * `request` - Pointer to a BatchEvalRequest structure containing:
///   - `expressions`: Array of expression strings to evaluate
///   - `expression_count`: Number of expressions
///   - `param_names`: Array of parameter names
///   - `param_count`: Number of parameters
///   - `param_values`: 2D array of parameter values [param_idx][batch_idx]
///   - `batch_size`: Number of items to evaluate
///   - `results`: 2D array to store results [expr_idx][batch_idx]
///   - `allocate_results`: Whether to allocate result arrays
///   - `stop_on_error`: Whether to stop on first error
///   - `statuses`: Optional array for error tracking
/// * `ctx` - Pointer to the evaluation context
///
/// # Returns
///
/// 0 on success, non-zero error code on failure:
/// - 1: NULL request or context
/// - 2: Invalid request (zero expressions or batch size)
/// - 3: NULL expression pointer
/// - 4: Invalid UTF-8 in expression
/// - 5: Expression parsing error
/// - 6: Evaluation error (when stop_on_error is true)
/// - 7: Memory allocation error
///
/// # Safety
///
/// This function is unsafe because it:
/// 1. Dereferences raw pointers
/// 2. Performs pointer arithmetic for array access
/// 3. Assumes arrays are properly sized as specified
///
/// The caller must ensure:
/// - All pointers are valid and properly aligned
/// - Arrays have the specified dimensions
/// - The context is valid and not freed during evaluation
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_eval(
    request: *const BatchEvalRequest,
    ctx: *mut EvalContextOpaque,
) -> i32 {
    use crate::engine::parse_expression;
    use crate::eval::iterative::{EvalEngine, eval_with_engine};

    // Safety checks
    if request.is_null() || ctx.is_null() {
        return 1;
    }

    let request = unsafe { &*request };

    // Validate request
    if request.expression_count == 0 || request.batch_size == 0 {
        return 2;
    }

    // Check results pointer
    if request.results.is_null() {
        return 2;
    }

    // Get context handle
    let ctx_handle = unsafe { &mut *(ctx as *mut alloc::rc::Rc<EvalContext>) };

    // Parse all expressions once
    let mut parsed_asts = Vec::with_capacity(request.expression_count);
    let mut parse_errors = Vec::with_capacity(request.expression_count);

    for i in 0..request.expression_count {
        let expr_ptr = unsafe { *request.expressions.add(i) };
        if expr_ptr.is_null() {
            if request.stop_on_error {
                return 3;
            }
            parsed_asts.push(None);
            parse_errors.push(Some(3));
            continue;
        }

        let expr_cstr = unsafe { CStr::from_ptr(expr_ptr) };
        let expr_str = match expr_cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                if request.stop_on_error {
                    return 4;
                }
                parsed_asts.push(None);
                parse_errors.push(Some(4));
                continue;
            }
        };

        // Parse expression (will use AST cache if enabled)
        match parse_expression(expr_str) {
            Ok(ast) => {
                parsed_asts.push(Some(ast));
                parse_errors.push(None);
            }
            Err(_) => {
                if request.stop_on_error {
                    return 5;
                }
                parsed_asts.push(None);
                parse_errors.push(Some(5));
            }
        }
    }

    // Use caller-provided results buffers
    let results_ptr = request.results;

    // Create a single engine for all evaluations
    let mut engine = EvalEngine::new();

    // Process each batch item
    for batch_idx in 0..request.batch_size {
        // Update all parameters using Rc::make_mut
        let ctx_mut = alloc::rc::Rc::make_mut(ctx_handle);

        for param_idx in 0..request.param_count {
            // Get parameter name
            let param_name_ptr = unsafe { *request.param_names.add(param_idx) };
            if param_name_ptr.is_null() {
                continue;
            }

            let param_cstr = unsafe { CStr::from_ptr(param_name_ptr) };
            let param_name = match param_cstr.to_str() {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Get parameter value
            let param_values_row = unsafe { *request.param_values.add(param_idx) };
            let param_value = unsafe { *param_values_row.add(batch_idx) };

            // Set parameter (ignore errors for now)
            let _ = ctx_mut.set_parameter(param_name, param_value);
        }

        // Evaluate all expressions with the same engine
        for (expr_idx, ast_opt) in parsed_asts.iter().enumerate() {
            // Check if we had a parse error for this expression
            if let Some(error_code) = parse_errors[expr_idx] {
                // Record error in status if tracking
                if !request.statuses.is_null() {
                    let status_idx = expr_idx * request.batch_size + batch_idx;
                    unsafe {
                        let status = &mut *request.statuses.add(status_idx);
                        status.code = error_code;
                        status.expr_index = expr_idx;
                        status.batch_index = batch_idx;
                    }
                }
                continue;
            }

            if let Some(ast) = ast_opt {
                // Use the shared context (just clone the Rc, which is cheap)
                let result = eval_with_engine(ast, Some(ctx_handle.clone()), &mut engine);

                match result {
                    Ok(value) => {
                        // Store result
                        unsafe {
                            let results_row = *results_ptr.add(expr_idx);
                            *results_row.add(batch_idx) = value;
                        }

                        // Update status if tracking
                        if !request.statuses.is_null() {
                            let status_idx = expr_idx * request.batch_size + batch_idx;
                            unsafe {
                                (*request.statuses.add(status_idx)).code = 0;
                            }
                        }
                    }
                    Err(_) => {
                        if request.stop_on_error {
                            return 6;
                        }

                        // Record error in status
                        if !request.statuses.is_null() {
                            let status_idx = expr_idx * request.batch_size + batch_idx;
                            unsafe {
                                let status = &mut *request.statuses.add(status_idx);
                                status.code = 7;
                                status.expr_index = expr_idx;
                                status.batch_index = batch_idx;
                            }
                        }
                    }
                }
            }
        }
    }

    // Note: The allocated results are already accessible through results_ptr which was used above

    0 // Success
}

/// Evaluates multiple expressions with batch allocation of results.
///
/// This is a convenience wrapper around exp_rs_batch_eval that handles
/// result allocation for you. The allocated results must be freed using
/// exp_rs_batch_free_results.
///
/// # Parameters
///
/// * `request` - Pointer to a BatchEvalRequest structure
/// * `ctx` - Pointer to the evaluation context
/// * `result` - Pointer to a BatchEvalResult structure to receive results
///
/// # Returns
///
/// 0 on success, non-zero error code on failure (same as exp_rs_batch_eval)
///
/// # Safety
///
/// This function has the same safety requirements as exp_rs_batch_eval.
/// Additionally, the caller must ensure that `result` points to valid memory.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_eval_alloc(
    request: *const BatchEvalRequest,
    ctx: *mut EvalContextOpaque,
    result: *mut BatchEvalResult,
) -> i32 {
    if request.is_null() || ctx.is_null() || result.is_null() {
        return 1;
    }

    // Create a modified request with our allocated buffers
    let mut modified_request = unsafe { *request };

    // Allocate result buffers for each expression
    let mut result_buffers: Vec<Vec<Real>> = Vec::with_capacity(modified_request.expression_count);
    for _ in 0..modified_request.expression_count {
        let mut buffer = Vec::with_capacity(modified_request.batch_size);
        buffer.resize(modified_request.batch_size, 0.0 as Real);
        result_buffers.push(buffer);
    }
    
    // Get pointers to each buffer
    let result_ptrs: Vec<*mut Real> = result_buffers.iter_mut()
        .map(|buf| buf.as_mut_ptr())
        .collect();
    
    // Leak both the buffers and the pointer array to keep them alive
    let leaked_buffers = Box::leak(result_buffers.into_boxed_slice());
    let leaked_ptrs = Box::leak(result_ptrs.into_boxed_slice());
    
    modified_request.results = leaked_ptrs.as_mut_ptr();

    // Call the main batch eval function
    let status = exp_rs_batch_eval(&modified_request, ctx);

    // Fill in the result structure
    unsafe {
        (*result).results = modified_request.results;
        (*result).expression_count = modified_request.expression_count;
        (*result).batch_size = modified_request.batch_size;
        (*result).status = status;
    }

    status
}

/// Frees results allocated by exp_rs_batch_eval_alloc.
///
/// This function releases memory allocated by the batch evaluation functions
/// when allocate_results was true or when using exp_rs_batch_eval_alloc.
///
/// # Parameters
///
/// * `result` - Pointer to a BatchEvalResult structure containing allocated results
///
/// # Safety
///
/// This function is unsafe because it:
/// 1. Dereferences a raw pointer
/// 2. Assumes the results were allocated by this library
///
/// The caller must ensure:
/// - The result pointer is valid
/// - The results were allocated by exp_rs_batch_eval_alloc
/// - The results have not already been freed
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_free_results(result: *mut BatchEvalResult) {
    if result.is_null() {
        return;
    }

    let result = unsafe { &mut *result };

    if !result.results.is_null() && result.expression_count > 0 {
        unsafe {
            // First, free each row of results
            let results_array =
                core::slice::from_raw_parts_mut(result.results, result.expression_count);

            for i in 0..result.expression_count {
                if !results_array[i].is_null() {
                    // Free the row (Vec<Real>)
                    let _ =
                        Vec::from_raw_parts(results_array[i], result.batch_size, result.batch_size);
                }
            }

            // Then free the array of row pointers
            let _ = Box::from_raw(result.results);
        }

        // Clear the result structure
        result.results = ptr::null_mut();
        result.expression_count = 0;
        result.batch_size = 0;
        result.status = 0;
    }
}

/// Batch evaluate multiple expressions with a pre-existing context
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_eval_with_context(
    request: *const BatchEvalRequest,
    ctx: *const EvalContextOpaque,
) -> i32 {
    use crate::engine::parse_expression;
    use crate::eval::iterative::{EvalEngine, eval_with_engine};
    use alloc::rc::Rc;
    if request.is_null() || ctx.is_null() {
        return -1;
    }

    let request = unsafe { &*request };

    // Validate request
    if request.expressions.is_null()
        || request.expression_count == 0
        || request.param_names.is_null()
        || request.param_values.is_null()
        || request.batch_size == 0
        || request.results.is_null()
    {
        return -1;
    }

    // Get the context
    let ctx_handle = unsafe { &*(ctx as *const alloc::rc::Rc<EvalContext>) };
    let mut base_ctx = ctx_handle.clone();

    // Create a single evaluation engine to reuse
    let mut engine = EvalEngine::new();

    let mut any_error = false;

    // Process each expression
    for expr_idx in 0..request.expression_count {
        // Get expression string
        let expr_ptr = unsafe { *request.expressions.add(expr_idx) };
        if expr_ptr.is_null() {
            if !request.statuses.is_null() {
                for batch_idx in 0..request.batch_size {
                    unsafe {
                        let status = &mut *request
                            .statuses
                            .add(expr_idx * request.batch_size + batch_idx);
                        status.code = -1;
                        status.expr_index = expr_idx;
                        status.batch_index = batch_idx;
                    }
                }
            }
            any_error = true;
            if request.stop_on_error {
                return -1;
            }
            continue;
        }

        let expr_cstr = unsafe { CStr::from_ptr(expr_ptr) };
        let expr_str = match expr_cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                if !request.statuses.is_null() {
                    for batch_idx in 0..request.batch_size {
                        unsafe {
                            let status = &mut *request
                                .statuses
                                .add(expr_idx * request.batch_size + batch_idx);
                            status.code = -2;
                            status.expr_index = expr_idx;
                            status.batch_index = batch_idx;
                        }
                    }
                }
                any_error = true;
                if request.stop_on_error {
                    return -2;
                }
                continue;
            }
        };

        // Parse the expression once
        let ast = match parse_expression(expr_str) {
            Ok(ast) => ast,
            Err(_) => {
                if !request.statuses.is_null() {
                    for batch_idx in 0..request.batch_size {
                        unsafe {
                            let status = &mut *request
                                .statuses
                                .add(expr_idx * request.batch_size + batch_idx);
                            status.code = -3;
                            status.expr_index = expr_idx;
                            status.batch_index = batch_idx;
                        }
                    }
                }
                any_error = true;
                if request.stop_on_error {
                    return -3;
                }
                continue;
            }
        };

        // Get result array pointer
        let result_ptr = unsafe { *request.results.add(expr_idx) };
        if result_ptr.is_null() {
            if !request.statuses.is_null() {
                for batch_idx in 0..request.batch_size {
                    unsafe {
                        let status = &mut *request
                            .statuses
                            .add(expr_idx * request.batch_size + batch_idx);
                        status.code = -4;
                        status.expr_index = expr_idx;
                        status.batch_index = batch_idx;
                    }
                }
            }
            any_error = true;
            if request.stop_on_error {
                return -4;
            }
            continue;
        }

        // Evaluate for each batch
        for batch_idx in 0..request.batch_size {
            // Update parameters in context using Rc::make_mut
            let ctx_mut = Rc::make_mut(&mut base_ctx);

            // Set parameters for this batch
            for param_idx in 0..request.param_count {
                let param_name_ptr = unsafe { *request.param_names.add(param_idx) };
                if param_name_ptr.is_null() {
                    continue;
                }

                let param_name_cstr = unsafe { CStr::from_ptr(param_name_ptr) };
                if let Ok(param_name) = param_name_cstr.to_str() {
                    // Get parameter values array for this parameter (using correct layout)
                    let param_values_row = unsafe { *request.param_values.add(param_idx) };
                    if !param_values_row.is_null() {
                        // Get value for this batch index
                        let param_value = unsafe { *param_values_row.add(batch_idx) };
                        ctx_mut.set_parameter(param_name, param_value);
                    }
                }
            }

            // Evaluate using the reusable engine
            match eval_with_engine(&ast, Some(base_ctx.clone()), &mut engine) {
                Ok(value) => {
                    unsafe {
                        *result_ptr.add(batch_idx) = value;
                    }
                    if !request.statuses.is_null() {
                        unsafe {
                            let status = &mut *request
                                .statuses
                                .add(expr_idx * request.batch_size + batch_idx);
                            status.code = 0;
                            status.expr_index = expr_idx;
                            status.batch_index = batch_idx;
                        }
                    }
                }
                Err(_) => {
                    if !request.statuses.is_null() {
                        unsafe {
                            let status = &mut *request
                                .statuses
                                .add(expr_idx * request.batch_size + batch_idx);
                            status.code = -5;
                            status.expr_index = expr_idx;
                            status.batch_index = batch_idx;
                        }
                    }
                    any_error = true;
                    if request.stop_on_error {
                        return -5;
                    }
                }
            }
        }
    }

    if any_error { -10 } else { 0 }
}

// ============================================================================
// Batch Builder API
// ============================================================================

use crate::batch_builder::BatchBuilder;

/// Opaque type for BatchBuilder
#[repr(C)]
pub struct BatchBuilderOpaque {
    _data: [u8; 0],
}

/// Creates a new batch builder for efficient expression evaluation.
///
/// The batch builder allows you to:
/// - Pre-parse expressions once
/// - Reuse a single evaluation engine
/// - Update parameters efficiently
/// - Evaluate all expressions in one call
///
/// # Returns
///
/// A pointer to the new batch builder, or NULL on allocation failure.
///
/// # Safety
///
/// The returned pointer must be freed with `exp_rs_batch_builder_free`.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_new() -> *mut BatchBuilderOpaque {
    let builder = Box::new(BatchBuilder::new());
    Box::into_raw(builder) as *mut BatchBuilderOpaque
}

/// Frees a batch builder.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Dereferences a raw pointer
/// - Frees memory allocated by Rust
///
/// The caller must ensure:
/// - The pointer was allocated by `exp_rs_batch_builder_new`
/// - The pointer is not used after this call
/// - The pointer is not freed multiple times
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_free(builder: *mut BatchBuilderOpaque) {
    if !builder.is_null() {
        unsafe {
            let _ = Box::from_raw(builder as *mut BatchBuilder);
        }
    }
}

/// Adds an expression to the batch builder.
///
/// The expression is parsed immediately and cached for efficient evaluation.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `expr` - The expression string to add
///
/// # Returns
///
/// The index of the added expression (>= 0) on success, or negative error code:
/// - `-1`: NULL pointer
/// - `-2`: Parse error
/// - `-3`: Invalid UTF-8 in expression
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure all pointers are valid.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_add_expression(
    builder: *mut BatchBuilderOpaque,
    expr: *const c_char,
) -> i32 {
    if builder.is_null() || expr.is_null() {
        return -1;
    }
    
    let builder = unsafe { &mut *(builder as *mut BatchBuilder) };
    let expr_cstr = unsafe { CStr::from_ptr(expr) };
    
    match expr_cstr.to_str() {
        Ok(expr_str) => match builder.add_expression(expr_str) {
            Ok(idx) => idx as i32,
            Err(_) => -2,
        },
        Err(_) => -3,
    }
}

/// Adds a parameter to the batch builder.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `name` - The parameter name
/// * `initial_value` - Initial value for the parameter
///
/// # Returns
///
/// The index of the added parameter (>= 0) on success, or negative error code:
/// - `-1`: NULL pointer
/// - `-2`: Duplicate parameter name or other error
/// - `-3`: Invalid UTF-8 in parameter name
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure all pointers are valid.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_add_parameter(
    builder: *mut BatchBuilderOpaque,
    name: *const c_char,
    initial_value: Real,
) -> i32 {
    if builder.is_null() || name.is_null() {
        return -1;
    }
    
    let builder = unsafe { &mut *(builder as *mut BatchBuilder) };
    let name_cstr = unsafe { CStr::from_ptr(name) };
    
    match name_cstr.to_str() {
        Ok(name_str) => match builder.add_parameter(name_str, initial_value) {
            Ok(idx) => idx as i32,
            Err(_) => -2,
        },
        Err(_) => -3,
    }
}

/// Sets a parameter value by index.
///
/// This is the fastest way to update parameter values.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `idx` - Parameter index (from `add_parameter`)
/// * `value` - New value for the parameter
///
/// # Returns
///
/// 0 on success, negative error code on failure:
/// - `-1`: NULL pointer
/// - `-2`: Invalid parameter index
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_set_param(
    builder: *mut BatchBuilderOpaque,
    idx: usize,
    value: Real,
) -> i32 {
    if builder.is_null() {
        return -1;
    }
    
    let builder = unsafe { &mut *(builder as *mut BatchBuilder) };
    match builder.set_param(idx, value) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

/// Sets a parameter value by name.
///
/// This is more convenient but slower than setting by index.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `name` - Parameter name
/// * `value` - New value for the parameter
///
/// # Returns
///
/// 0 on success, negative error code on failure:
/// - `-1`: NULL pointer
/// - `-2`: Unknown parameter name
/// - `-3`: Invalid UTF-8 in parameter name
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_set_param_by_name(
    builder: *mut BatchBuilderOpaque,
    name: *const c_char,
    value: Real,
) -> i32 {
    if builder.is_null() || name.is_null() {
        return -1;
    }
    
    let builder = unsafe { &mut *(builder as *mut BatchBuilder) };
    let name_cstr = unsafe { CStr::from_ptr(name) };
    
    match name_cstr.to_str() {
        Ok(name_str) => match builder.set_param_by_name(name_str, value) {
            Ok(_) => 0,
            Err(_) => -2,
        },
        Err(_) => -3,
    }
}

/// Evaluates all expressions with current parameter values.
///
/// This function updates the context with current parameter values and
/// evaluates all expressions using cached ASTs and a reusable engine.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `ctx` - Pointer to the evaluation context
///
/// # Returns
///
/// 0 on success, negative error code on failure:
/// - `-1`: NULL pointer
/// - `-2`: Evaluation error
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_eval(
    builder: *mut BatchBuilderOpaque,
    ctx: *mut EvalContextOpaque,
) -> i32 {
    if builder.is_null() || ctx.is_null() {
        return -1;
    }
    
    let builder = unsafe { &mut *(builder as *mut BatchBuilder) };
    let ctx_handle = unsafe { &*(ctx as *const alloc::rc::Rc<EvalContext>) };
    
    match builder.eval(ctx_handle) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

/// Gets the result of a specific expression by index.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
/// * `expr_idx` - Expression index (from `add_expression`)
///
/// # Returns
///
/// The result value, or NaN if the index is invalid.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_get_result(
    builder: *const BatchBuilderOpaque,
    expr_idx: usize,
) -> Real {
    if builder.is_null() {
        return Real::NAN;
    }
    
    let builder = unsafe { &*(builder as *const BatchBuilder) };
    builder.get_result(expr_idx).unwrap_or(Real::NAN)
}

/// Gets the number of parameters in the batch builder.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
///
/// # Returns
///
/// The number of parameters, or 0 if the pointer is NULL.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_param_count(
    builder: *const BatchBuilderOpaque,
) -> usize {
    if builder.is_null() {
        return 0;
    }
    
    let builder = unsafe { &*(builder as *const BatchBuilder) };
    builder.param_count()
}

/// Gets the number of expressions in the batch builder.
///
/// # Parameters
///
/// * `builder` - Pointer to the batch builder
///
/// # Returns
///
/// The number of expressions, or 0 if the pointer is NULL.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_batch_builder_expression_count(
    builder: *const BatchBuilderOpaque,
) -> usize {
    if builder.is_null() {
        return 0;
    }
    
    let builder = unsafe { &*(builder as *const BatchBuilder) };
    builder.expression_count()
}

