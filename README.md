# Teensycore

Teensycore is a kernel written in rust for the [Teensy-4.0 microcontroller](https://www.pjrc.com/store/teensy40.html).

## Installation

To properly build teensycore and any subsequent project, you'll need the following:

```bash
# Install build tools
sudo apt-get install gcc-arm-none-eabi jq

# Configure rust
rustup default nightly
rustup target add thumbv7em-none-eabi
```

## Usage

You must first configure your project as a library. Your Cargo.toml should look something like this:

```
[package]
name = "my_project"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]
path = "src/lib.rs"

[dependencies]
teensycore = "^0.0.9"
```

Teensycore exports a convenient macro that helps to configure the entrypoint of your application. It takes care of the default panic handler, initializing system clocks, setting up irq, enabling debug UART, and much more. In this way, you can just focus on what your project needs to get going. Replace your `src/lib.rs` with something like this:

```rust
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

teensycore::main!({
    /* Application code here */
});
```

You are now ready to start writing some baremetal rust!

## Building

In order for your project to build correctly, you'll need the following:

- Configure your project as a library
- Put your entrypoint code in src/lib.rs
- Download the build template [bash script](https://github.com/SharpCoder/teensycore/blob/main/build-template.sh)
- Execute `build-template.sh` in lieu of `cargo build`.

The build script will generate a `.hex` file and place it in a folder called `out`. This hex file is compatible with the teensy 4.0 and can be flashed with the teensy-loader utility.

**CAUTION**: Do not build this in release mode. It optimizes a lot of stuff away, and can cause problems.

## Example

Here is a very basic blinky example. To see more examples, check out the `/examples` folder.

```
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

use teensycore::*;
use teensycore::phys::pins::*;

main!({
    pin_mode(13, Mode::Output);

    loop {
        pin_out(13, Power::High);
        wait_ns(1 * S_TO_NANO);
        pin_out(13, Power::Low);
        wait_ns(1 * S_TO_NANO);
    }
});
```

## Contributing

This project is a work-in-progress and will be undergoing significant development over the coming months as I make it suitable for my own needs. Contributions are welcome.

## License

[MIT](https://choosealicense.com/licenses/mit/)
