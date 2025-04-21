/*!
 * C FFI interface for exp-rs.
 * This exposes a minimal, safe C API for evaluating expressions.
 * More functions can be added as needed.
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
#[repr(C)]
pub struct EvalResult {
    pub status: i32,          // 0 = OK, nonzero = error
    pub value: Real,          // valid if status == 0, using the Real type alias
    pub error: *const c_char, // valid if status != 0, must be freed by caller
}

/// Free a string allocated by exp_rs_eval error result.
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_free_error(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[unsafe(no_mangle)]
/// @return EvalResult structure
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

/// Opaque EvalContext handle for C
#[repr(C)]
pub struct EvalContextOpaque {
    _private: [u8; 0],
}

/// Create a new EvalContext and return a pointer to it
#[unsafe(no_mangle)]
pub extern "C" fn exp_rs_context_new() -> *mut EvalContextOpaque {
    let ctx = Box::new(EvalContext::new());
    Box::into_raw(ctx) as *mut EvalContextOpaque
}

/// Free an EvalContext previously created by exp_rs_context_new
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

#[unsafe(no_mangle)]
/// @return EvalResult structure
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
