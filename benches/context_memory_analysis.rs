use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use exp_rs::context::EvalContext;

// Custom allocator that tracks allocations
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static TRACKING_ENABLED: RefCell<bool> = RefCell::new(false);
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { System.alloc(layout) };
        
        TRACKING_ENABLED.with(|enabled| {
            if *enabled.borrow() && !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
                ALLOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        });
        
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TRACKING_ENABLED.with(|enabled| {
            if *enabled.borrow() {
                ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            }
        });
        
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    ALLOCATION_COUNT.store(0, Ordering::SeqCst);
}

fn start_tracking() {
    reset_tracking();
    TRACKING_ENABLED.with(|enabled| {
        *enabled.borrow_mut() = true;
    });
}

fn stop_tracking() -> (usize, usize) {
    TRACKING_ENABLED.with(|enabled| {
        *enabled.borrow_mut() = false;
    });
    (
        ALLOCATED.load(Ordering::SeqCst),
        ALLOCATION_COUNT.load(Ordering::SeqCst)
    )
}

fn measure_allocation<F: FnOnce() -> R, R>(name: &str, f: F) -> R {
    start_tracking();
    let result = f();
    let (bytes, count) = stop_tracking();
    println!("{:<50}: {:>8} bytes in {:>4} allocations", name, bytes, count);
    result
}

fn main() {
    println!("=== Context Memory Allocation Analysis ===\n");
    
    // Measure empty struct allocations
    println!("Basic Structure Allocations:");
    
    measure_allocation("Empty context (hypothetical)", || {
        // We can't create an empty context, but we can measure the difference
    });
    
    // Measure full context creation
    let ctx = measure_allocation("Full context with default functions", || {
        EvalContext::new()
    });
    
    println!("\nDetailed Component Analysis:");
    
    // Measure individual operations
    let mut ctx2 = ctx.clone();
    
    measure_allocation("Setting a parameter", || {
        ctx2.set_parameter("x", 1.0).unwrap();
    });
    
    measure_allocation("Setting 10 parameters", || {
        for i in 0..10 {
            ctx2.set_parameter(&format!("p{}", i), i as f64).unwrap();
        }
    });
    
    measure_allocation("Registering a native function", || {
        ctx2.register_native_function("test_func", 2, |args| args[0] + args[1]).unwrap();
    });
    
    measure_allocation("Registering an expression function", || {
        ctx2.register_expression_function("expr_func", &["x", "y"], "x + y").unwrap();
    });
    
    // Test AST caching
    println!("\nAST Cache Analysis:");
    
    let ctx3 = EvalContext::new();
    ctx3.enable_ast_cache();
    
    measure_allocation("Enabling AST cache", || {
        // Cache is already enabled, this should show 0
    });
    
    // Test context with different feature configurations
    println!("\nFeature Impact Analysis:");
    
    #[cfg(feature = "libm")]
    {
        println!("Built with libm feature - includes all math functions");
    }
    
    #[cfg(not(feature = "libm"))]
    {
        println!("Built without libm feature - basic functions only");
    }
    
    // Analyze the context structure
    println!("\nContext Structure Size Estimates:");
    println!("  VariableMap capacity: {}", exp_rs::types::EXP_RS_MAX_VARIABLES);
    println!("  ConstantMap capacity: {}", exp_rs::types::EXP_RS_MAX_CONSTANTS);
    println!("  ArrayMap capacity: {}", exp_rs::types::EXP_RS_MAX_ARRAYS);
    println!("  AttributeMap capacity: {}", exp_rs::types::EXP_RS_MAX_ATTRIBUTES);
    println!("  NativeFunctionMap capacity: {}", exp_rs::types::EXP_RS_MAX_NATIVE_FUNCTIONS);
    println!("  ExpressionFunctionMap capacity: {}", exp_rs::types::EXP_RS_MAX_EXPRESSION_FUNCTIONS);
    println!("  UserFunctionMap capacity: {}", exp_rs::types::EXP_RS_MAX_USER_FUNCTIONS);
    
    // Test creating multiple contexts to see if there's any sharing
    println!("\nMultiple Context Creation:");
    
    let ctx4 = measure_allocation("Second context creation", || {
        EvalContext::new()
    });
    
    let _ctx5 = measure_allocation("Third context creation", || {
        EvalContext::new()
    });
    
    // Test cloning
    println!("\nCloning Analysis:");
    
    let _ctx_clone = measure_allocation("Cloning a context", || {
        ctx4.clone()
    });
    
    // Function registration details
    println!("\nDefault Function Registration Details:");
    
    // Create a new context and track individual function registrations
    let test_ctx = EvalContext::new();
    
    // Try to count all registered functions
    let test_functions = vec![
        "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=",
        "&&", "||", "add", "sub", "mul", "div", "fmod", "neg",
        ",", "comma", "abs", "max", "min", "sign", "e", "pi",
        // libm functions
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
        "sinh", "cosh", "tanh", "exp", "ln", "log", "log10",
        "sqrt", "ceil", "floor"
    ];
    
    let mut registered = 0;
    for func_name in &test_functions {
        if test_ctx.get_native_function(func_name).is_some() {
            registered += 1;
        }
    }
    
    println!("Total functions checked: {}", test_functions.len());
    println!("Functions found: {}", registered);
    
    println!("\n=== Analysis Complete ===");
}