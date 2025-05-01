//! Simple test for verifying factorial recursion depth
//!
//! This test manually implements factorial recursion and verifies that
//! factorial(4) uses exactly 4 levels of call stack depth

/// Simple manual test for factorial recursion depth
#[test]
fn test_factorial_recursion_depth() {
    // Function to calculate factorial with recursion depth tracking
    fn factorial(n: u32, depth: u32) -> (u64, u32) {
        // Increment depth for this call
        let current_depth = depth + 1;
        
        // Print entry
        println!("{}{} -> factorial({}) [depth={}]", 
                 " ".repeat(current_depth as usize), 
                 current_depth, n, current_depth);
        
        // Calculate result and track max depth
        let (result, max_depth) = if n <= 1 {
            // Base case
            (1, current_depth)
        } else {
            // Recursive case
            let (sub_result, sub_depth) = factorial(n - 1, current_depth);
            (n as u64 * sub_result, sub_depth)
        };
        
        // Print exit
        println!("{}{} <- factorial({}) = {} [max_depth={}]", 
                 " ".repeat(current_depth as usize), 
                 current_depth, n, result, max_depth);
        
        // Return result and max depth
        (result, max_depth)
    }
    
    // Calculate factorial(4) and track the maximum recursion depth
    println!("\nCalculating factorial(4) with recursion depth tracking:");
    let (result, max_depth) = factorial(4, 0);
    
    // Verify correct result
    assert_eq!(result, 24, "factorial(4) should equal 24");
    
    // Verify exact recursion depth
    assert_eq!(max_depth, 4, "factorial(4) should use exactly 4 levels of recursion");
    
    println!("Test passed: factorial(4) = {} with {} levels of recursion", result, max_depth);
}