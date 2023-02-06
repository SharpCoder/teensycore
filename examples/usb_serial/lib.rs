//! Serial
//!
//! This example demonstrates how to output serial data over the wire.
//! It uses the built-in hardware-level UART controller in order to
//! transmit data.
//!
//! A fun way to verify this is working would be to hook up the RX pin
//! of an arduino to pin 1 on the teensy. Then you can open
//! "serial monitor" on your computer and watch the data coming
//! back from the teensy.
//!
//! Note: The default baud rate is configured to be 115200.
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

extern crate teensycore;

use teensycore::usb_serial::*;
use teensycore::*;

main!({
    serial_init(SerioDevice::Default);

    loop {
        usb_serial_write(b"ping!");
        wait_ns(S_TO_NANO);
    }
});
