use crate::{math::*, MS_TO_NANO};
use crate::clock::uNano;
use crate::phys::pins::*;
use crate::serio::*;
use crate::*;

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
    Fast = (crate::MS_TO_NANO * 350 as uNano) as isize,

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
    for _ in 0 .. count {
        blink_led_on();
        wait_ns(MS_TO_NANO * 250);
        blink_led_off();
        wait_ns(MS_TO_NANO * 150);
    }
}

const DEBUG_SERIAL_ENABLED: bool = true;

pub fn blink_custom(on_time: uNano, off_time: uNano) {
    blink_led_on();
    wait_ns(on_time);
    blink_led_off();
    wait_ns(off_time);
}

/// Write out a u32, in hex format, along with a string of data
/// to Uart4. This is useful for debugging memory addresses.
pub fn debug_hex(hex: u32, message: &[u8]) {
    if DEBUG_SERIAL_ENABLED {
        serial_write(SerioDevice::Debug, b"0x");
        serial_write_str(SerioDevice::Debug, &itob(hex as u64, 16));
        serial_write(SerioDevice::Debug, b" ");
        debug_str(message);
    }
}

/// Write out a u64 number and a string of data to Uart4
pub fn debug_u64(val: u64, message: &[u8]) {
    if DEBUG_SERIAL_ENABLED {
        serial_write_str(SerioDevice::Debug, &itoa(val));
        serial_write(SerioDevice::Debug, b" ");
        debug_str(message);
    }
}

/// Write out a string of data to Uart4
pub fn debug_str(message: &[u8]) {
    if DEBUG_SERIAL_ENABLED {
        serial_write(SerioDevice::Debug, message);
        serial_write(SerioDevice::Debug, b"\n");
    }
}