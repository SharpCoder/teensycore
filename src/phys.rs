//! Phys module handles kernel-level
//! interfacing for physical, on-board peripherals.

pub mod addrs;
pub mod analog;
pub mod dma;
pub mod gpio;
pub mod irq;
pub mod periodic_timers;
pub mod pins;
pub mod timer;
pub mod uart;
pub mod usb;
pub mod xbar;

pub enum Bitwise {
    Or,  // Or with the existing value
    And, // And with the existing value
    Eq,  // Assign absolute vlaue
}

pub enum Dir {
    Input,
    Output,
}

// Enable all physical clocks that we need
pub fn phys_clocks_en() {
    periodic_timers::pit_start_clock();
    uart::uart_start_clock();
    analog::analog_start_clock();
    gpio::gpio_start_clock();
    xbar::xbar_start_clock();
    dma::dma_start_clock();
    usb::usb_start_clock();
}

/// Takes a memory address and does an 8-bit write
/// to the location.
///
/// This method will be deprecated in the future,
/// in preference of `assign_8`
pub fn write_byte(address: u32, value: u8) {
    unsafe {
        *(address as *mut u8) = value;
    }
}

/// Takes a memory address and does an 8-bit write
/// to the location.
pub fn assign_8(address: u32, value: u8) {
    unsafe {
        *(address as *mut u8) = value;
    }
}

/// Takes a memory address and does a 16-bit write
/// to the location.
pub fn assign_16(address: u32, value: u16) {
    unsafe {
        *(address as *mut u16) = value;
    }
}

/// Takes a memory address and does a 32-bit write
/// to the location.
pub fn assign(address: u32, value: u32) {
    unsafe {
        *(address as *mut u32) = value;
    }
}

/// Takes a memory address, an operation, and a value
/// and performs the operations against the address.
///
/// This is useful if you want to maintain existing data
/// and logically AND or logically OR an additional byte.
pub fn assign_bit(address: u32, op: Bitwise, value: u32) {
    unsafe {
        let original_value = *(address as *mut u32);
        match op {
            Bitwise::Or => {
                assign(address, original_value | value);
            }
            Bitwise::And => {
                assign(address, original_value & value);
            }
            Bitwise::Eq => {
                assign(address, value);
            }
        }
    }
}

/// Takes a memory address and performs a 4-byte read,
/// resulting in a u32 of the current data.
pub fn read_word(address: u32) -> u32 {
    unsafe {
        return *(address as *mut u32);
    }
}

/// Takes a memory address and performs a 2-byte read,
/// resulting in a u16 of the current data.
pub fn read_16(address: u32) -> u16 {
    unsafe {
        return *(address as *mut u16);
    }
}

/// Takes a value and sets a particular bit to zero,
/// returning the new value.
pub fn clear_bit(number: u32, bit: u8) -> u32 {
    return number & !(0x01 << bit);
}

/// Takes a value and sets a particular bit to one,
/// returning the new value.
pub fn set_bit(number: u32, bit: u8) -> u32 {
    return number | (0x01 << bit);
}

/// A structure defining a register
/// used in peripherals
pub struct Reg {
    base: u32,
    mask: u32,
}
