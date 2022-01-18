//! Blinky
//! 
//! This example demonstrates how to blink "pin 13" which comes default
//! with the teensy-4.0 as the on-board orange LED. It relies on the
//! phys module which holds all the logic for interfacing with on-board
//! peripherals.
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

extern crate teensycore;

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