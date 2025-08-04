//! FFI-specific expression function validation tests
//! These tests verify the validation behavior of the C FFI interface

use exp_rs::ffi::*;
use std::ffi::{CString, c_char};
use std::ptr;

#[test]
fn test_ffi_null_pointer_validation() {
    unsafe {
        let ctx = exp_rs_context_new();
        assert!(!ctx.is_null());
        
        // Test null context
        let result = exp_rs_context_register_expression_function(
            ptr::null_mut(),
            CString::new("test").unwrap().as_ptr(),
            ptr::null(),
            0,
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 1);
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        // Test null function name
        let result = exp_rs_context_register_expression_function(
            ctx,
            ptr::null(),
            ptr::null(),
            0,
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 1);
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        // Test null expression
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
        );
        assert_eq!(result.status, 1);
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        exp_rs_context_free(ctx);
    }
}

#[test]
fn test_ffi_utf8_validation() {
    unsafe {
        let ctx = exp_rs_context_new();
        
        // Invalid UTF-8 in function name
        let invalid_utf8 = vec![0xFF, 0xFE, 0x00];
        let result = exp_rs_context_register_expression_function(
            ctx,
            invalid_utf8.as_ptr() as *const c_char,
            ptr::null(),
            0,
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 2); // Invalid UTF-8 in function name
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        // Invalid UTF-8 in expression
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            ptr::null(),
            0,
            invalid_utf8.as_ptr() as *const c_char,
        );
        assert_eq!(result.status, 3); // Invalid UTF-8 in expression
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        exp_rs_context_free(ctx);
    }
}

#[test]
fn test_ffi_parameter_validation() {
    unsafe {
        let ctx = exp_rs_context_new();
        
        // Create parameter array with null pointer
        let param1 = CString::new("x").unwrap();
        let param2_null: *const c_char = ptr::null();
        let params = vec![param1.as_ptr(), param2_null];
        
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            params.as_ptr(),
            2,
            CString::new("x + 1").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 4); // Null pointer in parameter list
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        // Test invalid UTF-8 in parameter
        let invalid_param = vec![0xFF, 0xFE, 0x00];
        let params = vec![invalid_param.as_ptr() as *const c_char];
        
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            params.as_ptr(),
            1,
            CString::new("x + 1").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 5); // Invalid UTF-8 in parameter name
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        exp_rs_context_free(ctx);
    }
}

#[test]
fn test_ffi_registration_errors() {
    unsafe {
        let ctx = exp_rs_context_new();
        
        // Test registration failure (e.g., parsing error)
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("bad_expr").unwrap().as_ptr(),
            ptr::null(),
            0,
            CString::new("(((").unwrap().as_ptr(), // Unmatched parentheses
        );
        assert_eq!(result.status, 6); // Failed to register expression function
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        // Test capacity exceeded - register 8 functions (the limit)
        for i in 0..8 {
            let name = CString::new(format!("func{}", i)).unwrap();
            let result = exp_rs_context_register_expression_function(
                ctx,
                name.as_ptr(),
                ptr::null(),
                0,
                CString::new("42").unwrap().as_ptr(),
            );
            assert_eq!(result.status, 0, "Failed to register function {}", i);
        }
        
        // The 9th should fail
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("func8").unwrap().as_ptr(),
            ptr::null(),
            0,
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 6); // Failed to register (capacity exceeded)
        assert!(!result.error.is_null());
        exp_rs_free_error(result.error as *mut c_char);
        
        exp_rs_context_free(ctx);
    }
}

#[test]
fn test_ffi_parameter_count_mismatch() {
    unsafe {
        let ctx = exp_rs_context_new();
        
        // Create parameter array
        let param1 = CString::new("x").unwrap();
        let param2 = CString::new("y").unwrap();
        let params = vec![param1.as_ptr(), param2.as_ptr()];
        
        // Register with correct count
        let result = exp_rs_context_register_expression_function(
            ctx,
            CString::new("add").unwrap().as_ptr(),
            params.as_ptr(),
            2, // Correct count
            CString::new("x + y").unwrap().as_ptr(),
        );
        assert_eq!(result.status, 0);
        
        // Note: Currently there's no validation that param_count matches actual array size
        // This could lead to undefined behavior if param_count > actual array size
        
        exp_rs_context_free(ctx);
    }
}