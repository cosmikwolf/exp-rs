fn main() {
    // evaluate expression and fetch result
    let result = exp_rs::interp("2*1/sin(pi/2)", None).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    println!("{:?}", result);
}
