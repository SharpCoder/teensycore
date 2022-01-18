//! Debug
//! 
//! This example demonstrates how to output debug data over serial.
//! It uses the built-in hardware-level UART controller in order to
//! transmit data.
//! 
//! The debug module reserves UART4 as the designated peripheral for
//! all serial communication. A good way to verify this is working 
//! would be to hook up the RX pin of an arduino to pin 8 on the teensy. 
//! Then you can open "serial monitor" on your computer and watch the data 
//! coming back from the teensy.
//! 
//! The debug module provides some convenience methods for transmitting
//! numbers, hex values, and ascii byte arrays..
//! 
//! Note: The default baud rate is 115200

extern crate teensycore;

use teensycore::*;
use teensycore::debug::*;

main!({
    let mut count = 0;

    loop {
        debug_u32(count, b"iteration");
        debug_hex(0xff72, b"hex values");
        debug_str(b"debug ping!");
        wait_ns(S_TO_NANO);

        count += 1;
    }
});