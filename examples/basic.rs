fn main() {
    // Evaluate expression with basic math operations (no special functions)
    // This should work with or without libm
    let result1 = exp_rs::interp("2 + 3 * 4", None).unwrap_or_else(|e| {
        panic!("{}", e);
    });
    println!("2 + 3 * 4 = {:?}", result1);

    // If libm is enabled, also test expressions with transcendental functions
    #[cfg(feature = "libm")]
    {
        let result2 = exp_rs::interp("2*1/sin(pi/2)", None).unwrap_or_else(|e| {
            panic!("{}", e);
        });
        println!("2*1/sin(pi/2) = {:?}", result2);
    }
}
