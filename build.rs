fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = "include";

    // Ensure the include directory exists
    std::fs::create_dir_all(out_dir).expect("Failed to create include directory");

    let header_path = std::path::Path::new(&out_dir).join("exp_rs.h");

    // Create a config for cbindgen
    let mut config =
        cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen.toml");

    // Set the define based on which feature is enabled
    // We only want to define one of these at a time to avoid C compilation errors
    if std::env::var("CARGO_FEATURE_F64").is_ok() {
        let _ = config.after_includes.insert("#define USE_F64".to_string());
    } else if std::env::var("CARGO_FEATURE_F32").is_ok() {
        let _ = config.after_includes.insert("#define USE_F32".to_string());
    }

    if std::env::var("CARGO_FEATURE_USE_CUSTOM_ALLOC").is_ok() {
        let _ = config
            .after_includes
            .insert("#define EXP_RS_USE_CUSTOM_ALLOC".to_string());
    }
    // Add FreeRTOS support if the freertos feature is enabled
    if std::env::var("CARGO_FEATURE_FREERTOS").is_ok() {
        let _ = config
            .after_includes
            .insert("#define USE_FREERTOS".to_string());

        // Add FreeRTOS function declarations
        let freertos_funcs = r#"
extern void *pvPortMalloc(uintptr_t size);

extern void vPortFree(void *ptr);
"#;
        let _ = config.after_includes.insert(freertos_funcs.to_string());
    }

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
