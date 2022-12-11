//! Analog to digital peripheral. There are 9 ADC pins
//! located on the Teensy4.0 and this module exposes
//! the ability to interact with them.
//!
//! ```no_run
//! use teensycore::phys::analog::*;
//!
//! // Read pin 20 (A6)
//! let val = analog_read(20);
//! ```
//!
use crate::assembly;

use super::{addrs, assign, assign_bit, read_word, Bitwise};

use core::arch::asm;

pub enum Resolution {
    Bits8 = 0x0,
    Bits10 = 0x1,
    Bits12 = 0x2,
}

/** The index is an arduino analog pin (0-9) the value is corresponding to the IOMUX register */
const ANALOG_PIN_BITS: [u32; 10] = [7, 8, 12, 11, 6, 5, 15, 0, 13, 14];

/// Start the ADC1 clock and configure it with some default resolution.
pub fn analog_start_clock() {
    assign_bit(addrs::CCM_CCGR1, Bitwise::Or, 0x3 << 16);

    // Default to 10-bits
    analog_set_resolution(Resolution::Bits10);
}

/// Configure the ADC resolution. Set to either 8, 10, or 12 bits.
pub fn analog_set_resolution(resolution: Resolution) {
    assign(0x400C_4044, (0x1 << 9) | (resolution as u32) << 2);
}

/// Read from the ADC
///
/// pin is the Arduino Pin as referenced from the pinout. For example
/// Pin 20 is the A6 pin.
pub fn analog_read(pin: usize) -> u32 {
    if pin > 23 || pin < 14 {
        // Error condition
        return 0;
    }

    // Enable the ADC for the specified pin
    let analog_idx = pin - 14;
    assign(addrs::ADC1_HC0, ANALOG_PIN_BITS[analog_idx]);

    // Wait until value is ready
    // TODO: This could loop forever?
    loop {
        let val = read_word(addrs::ADC1_HS);
        if val & 0x1 > 0 {
            break;
        } else {
            assembly!("nop");
        }
    }

    // Transfer data
    return read_word(0x400C_4024);
}
