//! Foreign Function Interface (FFI) for C/C++ interoperability
//!
//! This module provides a simplified C API for expression evaluation with arena-based memory management.

use crate::{AstExpr, EvalContext, Real};
use crate::error::ExprError;
use crate::expression::{Expression, Param, ArenaBatchBuilder};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::string::ToString;
use alloc::vec::Vec;
use bumpalo::Bump;
use core::ffi::{c_char, c_void, CStr};
use core::ptr;
use core::slice;

pub mod arena_pool;
use self::arena_pool::global_pool;

// Re-export for external visibility
pub use crate::expression::ArenaBatchBuilder as ArenaBatchBuilderExport;
pub use crate::expression::Expression as ExpressionExport;

// ============================================================================
// Error Handling
// ============================================================================

/// Result structure for FFI operations
#[repr(C)]
pub struct ExprResult {
    /// 0 for success, non-zero for error
    status: i32,
    /// Result value (NaN on error)
    value: Real,
    /// Error message (NULL on success, must be freed with expr_free_error)
    error: *mut c_char,
}

/// Free an error message string
///
/// # Safety
/// The pointer must have been returned by an expr_* function
#[unsafe(no_mangle)]
pub extern "C" fn expr_free_error(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

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
    if ctx.is_null() || name.is_null() || func as *const c_void == ptr::null() {
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

/// Add an expression to the batch
///
/// # Parameters
/// - `batch`: The batch
/// - `expr`: Expression string (must be valid UTF-8)
///
/// # Returns
/// Expression index on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_add_expression(
    batch: *mut ExprBatch,
    expr: *const c_char,
) -> i32 {
    if batch.is_null() || expr.is_null() {
        return -1;
    }

    let builder = unsafe { &mut *(batch as *mut ArenaBatchBuilder) };
    
    let expr_cstr = unsafe { CStr::from_ptr(expr) };
    let expr_str = match expr_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid UTF-8
    };

    match builder.add_expression(expr_str) {
        Ok(idx) => idx as i32,
        Err(_) => -3, // Parse error
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
/// Variable index on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn expr_batch_add_variable(
    batch: *mut ExprBatch,
    name: *const c_char,
    value: Real,
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

    match builder.add_parameter(name_str, value) {
        Ok(idx) => idx as i32,
        Err(_) => -3, // Error (e.g., duplicate name)
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
pub extern "C" fn expr_batch_set_variable(
    batch: *mut ExprBatch,
    index: usize,
    value: Real,
) -> i32 {
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
pub extern "C" fn expr_batch_evaluate(
    batch: *mut ExprBatch,
    ctx: *mut ExprContext,
) -> i32 {
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
pub extern "C" fn expr_batch_get_result(
    batch: *const ExprBatch,
    index: usize,
) -> Real {
    if batch.is_null() {
        return Real::NAN;
    }

    let builder = unsafe { &*(batch as *const ArenaBatchBuilder) };
    builder.get_result(index).unwrap_or(Real::NAN)
}

// ============================================================================
// Session API (Single Expression Evaluation with Arena Pool)
// ============================================================================

// Safe wrapper around Expression data without lifetime parameters
struct ExpressionWrapper {
    // Store the arena checkout
    checkout: arena_pool::ArenaCheckout,
    // Store expression strings
    expressions: Vec<String>,
    // Store parameter names and values  
    params: Vec<(String, Real)>,
    // Store results
    results: Vec<Real>,
}

impl ExpressionWrapper {
    /// Build a temporary Expression for evaluation
    fn with_expression<F, R>(&mut self, ctx: &alloc::rc::Rc<EvalContext>, f: F) -> Result<R, crate::error::ExprError>
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
    arena_pool::initialize(max_arenas).is_ok()
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
pub extern "C" fn expr_session_parse(
    session: *mut ExprSession,
    expr: *const c_char,
) -> i32 {
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