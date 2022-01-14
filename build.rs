// Example custom build script.
fn main() {
    cc::Build::new()
        .file("src/teensy.c")
        .opt_level(3)
        .compile("teensy");
}