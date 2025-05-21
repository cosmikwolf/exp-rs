use exp_rs::engine::interp;

fn main() {
    println!("Testing @ symbol parsing...");
    
    // Test cases
    let test_cases = [
        "1",
        "1 @ 2", 
        "1 $ 2",
        "1 # 2",
    ];
    
    for expr in test_cases {
        match interp(expr, None) {
            Ok(result) => println!("{} => {}", expr, result),
            Err(e) => println!("{} => ERROR: {}", expr, e),
        }
    }
}