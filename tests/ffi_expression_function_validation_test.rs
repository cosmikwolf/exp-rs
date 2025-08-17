//! FFI-specific expression function validation tests
//! These tests verify the validation behavior of the C FFI interface

use exp_rs::ffi::*;
use std::ffi::{CString, c_char};
use std::ptr;

#[test]
fn test_ffi_null_pointer_validation() {
    unsafe {
        let ctx = expr_context_new();
        assert!(!ctx.is_null());

        // Test null context
        let result = expr_context_add_expression_function(
            ptr::null_mut(),
            CString::new("test").unwrap().as_ptr(),
            CString::new("").unwrap().as_ptr(),
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result, -1); // Expected error code for null context

        // Test null function name
        let result = expr_context_add_expression_function(
            ctx,
            ptr::null(),
            CString::new("").unwrap().as_ptr(),
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result, -1); // Expected error code for null name

        // Test null parameters
        let result = expr_context_add_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            ptr::null(),
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result, -1); // Expected error code for null params

        // Test null expression
        let result = expr_context_add_expression_function(
            ctx,
            CString::new("test").unwrap().as_ptr(),
            CString::new("").unwrap().as_ptr(),
            ptr::null(),
        );
        assert_eq!(result, -1); // Expected error code for null expression

        expr_context_free(ctx);
    }
}

#[test]
fn test_ffi_invalid_utf8_validation() {
    unsafe {
        let ctx = expr_context_new();
        assert!(!ctx.is_null());

        // Create invalid UTF-8 string
        let invalid_utf8 = b"invalid\xff\xfe";
        let name_ptr = invalid_utf8.as_ptr() as *const c_char;

        let result = expr_context_add_expression_function(
            ctx,
            name_ptr,
            CString::new("").unwrap().as_ptr(),
            CString::new("42").unwrap().as_ptr(),
        );
        assert_eq!(result, -2); // Expected error code for invalid UTF-8

        expr_context_free(ctx);
    }
}

#[test]
fn test_ffi_successful_registration() {
    unsafe {
        let ctx = expr_context_new();
        assert!(!ctx.is_null());

        // Test successful registration
        let result = expr_context_add_expression_function(
            ctx,
            CString::new("double").unwrap().as_ptr(),
            CString::new("x").unwrap().as_ptr(),
            CString::new("x * 2").unwrap().as_ptr(),
        );
        assert_eq!(result, 0); // Success

        expr_context_free(ctx);
    }
}

#[test]
fn test_ffi_invalid_expression_syntax() {
    unsafe {
        let ctx = expr_context_new();
        assert!(!ctx.is_null());

        // Test with invalid expression syntax
        let result = expr_context_add_expression_function(
            ctx,
            CString::new("invalid_func").unwrap().as_ptr(),
            CString::new("x").unwrap().as_ptr(),
            CString::new("x + * 2").unwrap().as_ptr(), // Invalid syntax
        );
        assert_eq!(result, -3); // Expected error code for registration failure

        expr_context_free(ctx);
    }
}

#[test]
fn test_ffi_remove_expression_function() {
    unsafe {
        let ctx = expr_context_new();
        assert!(!ctx.is_null());

        // First add a function
        let result = expr_context_add_expression_function(
            ctx,
            CString::new("square").unwrap().as_ptr(),
            CString::new("x").unwrap().as_ptr(),
            CString::new("x * x").unwrap().as_ptr(),
        );
        assert_eq!(result, 0); // Success

        // Now remove it
        let result =
            expr_context_remove_expression_function(ctx, CString::new("square").unwrap().as_ptr());
        assert_eq!(result, 1); // Function was removed

        // Try to remove non-existent function
        let result = expr_context_remove_expression_function(
            ctx,
            CString::new("nonexistent").unwrap().as_ptr(),
        );
        assert_eq!(result, 0); // Function didn't exist

        expr_context_free(ctx);
    }
}
