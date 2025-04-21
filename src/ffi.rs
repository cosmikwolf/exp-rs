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

    // Simple wrapper for C stdlib malloc/free
    struct CAllocator;

    unsafe extern "C" {
        // Use void* type compatibility with stdlib.h
        fn malloc(size: usize) -> *mut core::ffi::c_void;
        fn free(ptr: *mut core::ffi::c_void);
    }

    unsafe impl GlobalAlloc for CAllocator {
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

    #[global_allocator]
    static ALLOCATOR: CAllocator = CAllocator;
}

// Panic handler for no_std
#[cfg(all(not(test), target_arch = "arm"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use alloc::ffi::CString;
use core::ptr;
use crate::Real;

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
/// Returns 0 on success, nonzero on error.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_register_expression_function(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
    params: *const *const c_char,
    param_count: usize,
    expression: *const c_char,
) -> i32 {
    if ctx.is_null() || name.is_null() || expression.is_null() {
        return 1;
    }
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return 2,
    };

    let expr_cstr = unsafe { CStr::from_ptr(expression) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return 3,
    };

    let mut param_vec = Vec::new();
    if !params.is_null() {
        for i in 0..param_count {
            let param_ptr = unsafe { *params.add(i) };
            if param_ptr.is_null() {
                return 4;
            }
            let param_cstr = unsafe { CStr::from_ptr(param_ptr) };
            match param_cstr.to_str() {
                Ok(s) => param_vec.push(s.to_string()),
                Err(_) => return 5,
            }
        }
    }

    match ctx.register_expression_function(
        name_str,
        &param_vec.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        expr_str,
    ) {
        Ok(_) => 0,
        Err(_) => 6,
    }
}

/// Set a parameter value in the context
/// Returns 0 on success, nonzero on error
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_set_parameter(
    ctx: *mut EvalContextOpaque,
    name: *const c_char,
    value: Real,
) -> i32 {
    if ctx.is_null() || name.is_null() {
        return 1;
    }
    let ctx = unsafe { &mut *(ctx as *mut EvalContext) };

    let name_cstr = unsafe { CStr::from_ptr(name) };
    let name_str = match name_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return 2,
    };

    ctx.set_parameter(name_str, value);
    0
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
