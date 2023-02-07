//! Serial
//!
//! This example demonstrates how to output serial data over USB.
//! It uses the built-in hardware-level USB OTG controller in order to
//! transmit data.
//!
//! Specifically, this code will wait to receive some bytes from the
//! USB device and, if any are found, it will echo them back. You can
//! actually use the Arduino Serial Monitor to echo commands from the
//! input box back out to the terminal.
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

extern crate teensycore;

use teensycore::usb_serial::*;
use teensycore::*;

main!({
    loop {
        match usb_serial_read() {
            Some(byte) => {
                usb_serial_putchar(byte);
            }
            None => {}
        }
    }
});
