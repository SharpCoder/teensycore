use std::env;

// Example custom build script.
fn main() {
    let tests_enabled = env::var_os("CARGO_FEATURE_TESTING").is_some();

    if !tests_enabled {
        cc::Build::new()
            .file("src/teensy.c")
            .opt_level(3)
            .flag("-Wall")
            .flag("-mcpu=cortex-m7")
            .flag("-mthumb")
            .flag("-mfloat-abi=hard")
            .compile("teensy");
    }
}
