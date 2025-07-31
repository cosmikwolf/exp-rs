fn main() {
    // Skip FFI build if requested
    if std::env::var("SKIP_FFI_BUILD").is_ok() {
        println!("cargo:warning=Skipping FFI build due to SKIP_FFI_BUILD environment variable");
        return;
    }

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = "include";

    // Ensure the include directory exists
    std::fs::create_dir_all(out_dir).expect("Failed to create include directory");

    let header_path = std::path::Path::new(&out_dir).join("exp_rs.h");

    // Create a config for cbindgen
    let mut config =
        cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen.toml");

    let mut after_includes_string = Vec::new();

    // after_includes_string.push(r#"#ifdef __cplusplus"#.to_string());
    // after_includes_string.push(r#"extern "C" {"#.to_string());
    // after_includes_string.push(r#"#endif"#.to_string());
    //

    config.cpp_compat = true;
    // Set the define based on which feature is enabled
    // We only want to define one of these at a time to avoid C compilation errors
    if std::env::var("CARGO_FEATURE_F64").is_ok() {
        after_includes_string.push("#define USE_F64".to_string());
    } else if std::env::var("CARGO_FEATURE_F32").is_ok() {
        after_includes_string.push("#define USE_F32".to_string());
    }
    if std::env::var("CARGO_FEATURE_CUSTOM_CBINDGEN_ALLOC").is_ok() {
        after_includes_string.push("#define EXP_RS_CUSTOM_ALLOC".to_string());
    }

    let _ = config
        .after_includes
        .insert(after_includes_string.join("\n"));
    // Add a custom prefix to the header with our type definitions
    // let mut prefix = String::new();
    // if std::env::var("CARGO_FEATURE_F32").is_ok() {
    //     prefix.push_str("#define TEST_PRECISION 1e-6\n");
    // } else if std::env::var("CARGO_FEATURE_F64").is_ok() {
    //     prefix.push_str("#define TEST_PRECISION 1e-10\n");
    // }
    // config.header = Some(prefix);

    match cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(header_path);
        }
        Err(e) => {
            eprintln!("Unable to generate bindings: {e}");
            eprintln!(
                "Hint: check crate-level doc comments in src/lib.rs (should be a plain string literal)"
            );
            std::process::exit(1);
        }
    }
}
