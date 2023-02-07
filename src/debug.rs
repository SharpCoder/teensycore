use crate::clock::uNano;
use crate::phys::pins::*;
use crate::serio::*;
use crate::system::str::*;
use crate::system::vector::{Stack, Vector};
use crate::usb_serial::*;
use crate::*;
use crate::{math::*, MS_TO_NANO};

const UART_SERIAL: bool = true;
const USB_SERIAL: bool = true;

#[derive(Copy, Clone)]
pub struct BlinkConfig {
    pub speed: Speed,
    pub remaining_count: u8,
}

#[derive(Copy, Clone)]
pub enum Speed {
    /// One second
    Slow = (crate::MS_TO_NANO * 1000 as uNano) as isize,

    /// 350ms
    Fast = (crate::MS_TO_NANO * 150 as uNano) as isize,

    /// 700ms
    Normal = (crate::MS_TO_NANO * 700 as uNano) as isize,
}

pub static mut BLINK_CONFIG: BlinkConfig = BlinkConfig {
    speed: Speed::Normal,
    remaining_count: 0,
};

/// Turn LED 13 on.
pub fn blink_led_on() {
    pin_out(13, Power::High);
}

/// Turn LED 13 off.
pub fn blink_led_off() {
    pin_out(13, Power::Low);
}

/// Blink LED 13 a particular number of times
/// at a particular speed.
pub fn blink(count: u8, speed: Speed) {
    unsafe {
        if BLINK_CONFIG.remaining_count == 0 || speed as isize != BLINK_CONFIG.speed as isize {
            BLINK_CONFIG.speed = speed;
            BLINK_CONFIG.remaining_count = count * 2; // Multiply by two, one for each blink state
        }
    }
}

/// This will add 1 to whatever blink count is currently
/// active.
pub fn blink_accumulate() {
    unsafe {
        BLINK_CONFIG.speed = Speed::Slow;
        BLINK_CONFIG.remaining_count += 2;
    }
}

/// This method will flash LED 13 using hardware-level waits
/// (hard wait) instead of relying on Gates.
pub fn blink_hardware(count: u8) {
    for _ in 0..count {
        blink_led_on();
        wait_ns(MS_TO_NANO * 250);
        blink_led_off();
        wait_ns(MS_TO_NANO * 150);
    }
}

pub fn blink_custom(on_time: uNano, off_time: uNano) {
    blink_led_on();
    wait_ns(on_time);
    blink_led_off();
    wait_ns(off_time);
}

/// Print an f32 (with two decimal places) to Uart4
/// With no additional modifications.
pub fn print_f32(val: f32) {
    let mut bytes = Vector::<u8>::new();
    let mut num = val;
    let negative = val < 0.0;
    if negative {
        num = num * -1.0;
        bytes.push(b'-');
    }

    let major = num as u32;
    let decimal = ((num * 100.0) as u32) - major * 100;

    // Calculate the major value
    let mut major_str = itoa(major as u64);
    let mut major_vec = major_str.as_vector();

    let mut minor_str = itoa(decimal as u64);
    let mut minor_vec = minor_str.as_vector();

    bytes.join(&major_vec);
    bytes.push(b'.');
    bytes.join(&minor_vec);

    for byte in bytes.into_iter() {
        if UART_SERIAL {
            serial_write(SerioDevice::Debug, &[byte]);
        }

        if USB_SERIAL {
            usb_serial_putchar(byte);
        }
    }

    major_vec.free();
    minor_vec.free();
    major_str.drop();
    minor_str.drop();
    bytes.free();
}

pub fn print(message: &[u8]) {
    if UART_SERIAL {
        serial_write(SerioDevice::Debug, message);
    }

    if USB_SERIAL {
        usb_serial_write(message);
    }
}

/// Write out a u32, in hex format, along with a string of data
/// to Uart4. This is useful for debugging memory addresses.
pub fn debug_binary(hex: u32, message: &[u8]) {
    let mut bytes = Vector::<u8>::new();
    bytes.push(b'0');
    bytes.push(b'b');

    let str = &mut itob(hex as u64, 2);
    let mut vec = str.as_vector();
    bytes.join(&vec);
    bytes.push(b' ');

    for byte in bytes.into_iter() {
        if UART_SERIAL {
            serial_write(SerioDevice::Debug, &[byte]);
        }

        if USB_SERIAL {
            usb_serial_putchar(byte);
        }
    }

    debug_str(message);

    bytes.free();
    vec.free();
    str.drop();
}

/// Write out a u32, in hex format, along with a string of data
/// to Uart4. This is useful for debugging memory addresses.
pub fn debug_hex(hex: u32, message: &[u8]) {
    let mut bytes = Vector::<u8>::new();
    bytes.push(b'0');
    bytes.push(b'x');

    let str = &mut itob(hex as u64, 16);
    let mut vec = str.as_vector();
    bytes.join(&vec);
    bytes.push(b' ');

    for byte in bytes.into_iter() {
        if UART_SERIAL {
            serial_write(SerioDevice::Debug, &[byte]);
        }

        if USB_SERIAL {
            usb_serial_putchar(byte);
        }
    }

    debug_str(message);

    bytes.free();
    vec.free();
    str.drop();
}

/// Write out a u64 number and a string of data to Uart4
pub fn debug_u64(val: u64, message: &[u8]) {
    let mut bytes = Vector::<u8>::new();
    let str = &mut itoa(val);
    let mut vec = str.as_vector();
    bytes.join(&vec);
    bytes.push(b' ');

    for byte in bytes.into_iter() {
        if UART_SERIAL {
            serial_write(SerioDevice::Debug, &[byte]);
        }

        if USB_SERIAL {
            usb_serial_putchar(byte);
        }
    }

    debug_str(message);

    bytes.free();
    vec.free();
    str.drop();
}

/// Write out an f32 (with two decimal places) to Uart4
pub fn debug_f32(val: f32, message: &[u8]) {
    // Calculate the major value
    print_f32(val);

    if UART_SERIAL {
        serial_write(SerioDevice::Debug, b" ");
    }
    if USB_SERIAL {
        usb_serial_putchar(b' ');
    }

    debug_str(message);
}

/// Write out a string of data to Uart4
pub fn debug_str(message: &[u8]) {
    if UART_SERIAL {
        serial_write(SerioDevice::Debug, message);
        serial_write(SerioDevice::Debug, b"\n");
    }

    if USB_SERIAL {
        usb_serial_write(message);
        usb_serial_write(b"\n");
    }
}
