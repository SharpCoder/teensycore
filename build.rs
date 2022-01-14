// Example custom build script.
fn main() {
    cc::Build::new()
        .file("src/teensy.c")
        .opt_level(3)
        .flag("-Wall")
        .flag("-mcpu=cortex-m7")
        .flag("-mthumb")
        .flag("-mfloat-abi=hard")
    .compile("teensy");
}