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
            if size == 0 {
                return layout.align() as *mut u8;
            }
            // Cast the void* to u8*
            unsafe { exp_rs_malloc(size) as *mut u8 }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            if !ptr.is_null() {
                // Cast the u8* to void*
                unsafe { exp_rs_free(ptr as *mut core::ffi::c_void) };
            }
        }
    }

    // Implementation for standard allocator (using malloc/free)
    #[cfg(not(feature = "custom_cbindgen_alloc"))]
    unsafe impl GlobalAlloc for StandardAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            if size == 0 {
                return layout.align() as *mut u8;
            }
            // Cast the void* to u8*
            unsafe { malloc(size) as *mut u8 }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
            if !ptr.is_null() {
                // Cast the u8* to void*
                unsafe { free(ptr as *mut core::ffi::c_void) };
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
    let ctx = Box::new(EvalContext::new());
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
        // Need to drop the Box explicitly
        let _ = Box::from_raw(ctx as *mut EvalContext);
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
    
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };

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
                let msg = CString::new(format!("Null pointer in parameter list at index {}", i)).unwrap();
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
                    let msg = CString::new(format!("Invalid UTF-8 in parameter name at index {}: {}", i, e)).unwrap();
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

    match ctx.register_expression_function(
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
            let msg = CString::new(format!("Failed to register expression function: {}", e)).unwrap();
            let ptr = msg.into_raw();
            EvalResult {
                status: 6,
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
    
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };

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
    ctx.register_native_function(name_str, arity, implementation);

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
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };

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

    ctx.set_parameter(name_str, value);
    
    EvalResult {
        status: 0,
        value: value,  // Return the value that was set
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
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };
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
    let ctx_rc = alloc::rc::Rc::new(ctx.clone());
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