pub mod addrs;
pub mod dma;
pub mod gpio;
pub mod irq;
pub mod periodic_timers;
pub mod pins;
pub mod timer;
pub mod uart;
pub mod xbar;

pub enum Bitwise {
    Or, // Or with the existing value
    And, // And with the existing value
    Eq, // Assign absolute vlaue
}

pub enum Dir {
    Input,
    Output,
}

// Enable all physical clocks that we need
pub fn phys_clocks_en() {
    gpio::gpio_start_clock();
    uart::uart_start_clock();
    dma::dma_start_clock();
    xbar::xbar_start_clock();
    periodic_timers::pit_start_clock();
}

pub fn write_byte(address: u32, value: u8) {
    unsafe {
        *(address as *mut u8) = value;
    }
}

pub fn assign_8(address: u32, value: u8) {
    unsafe {
        *(address as *mut u8) = value;
    }
}

pub fn assign_16(address: u32, value: u16) {
    unsafe {
        *(address as *mut u16) = value;
    }
}

pub fn assign(address: u32, value: u32) {
    unsafe {
        *(address as *mut u32) = value;
    }
}

pub fn assign_bit(address: u32, op: Bitwise, value: u32) {
    unsafe {
        let original_value = *(address as *mut u32);
        match op {
            Bitwise::Or => {
                assign(address, original_value | value);
            },
            Bitwise::And => {
                assign(address, original_value & value);
            },
            Bitwise::Eq => {
                assign(address, value);
            }
        }
    }
}

pub fn read_word(address: u32) -> u32 {
    unsafe {
        return *(address as *mut u32);
    }
}

pub fn read_16(address: u32) -> u16 {
    unsafe {
        return *(address as *mut u16);
    }
}

pub fn clear_bit(number: u32, bit: u8) -> u32 {
    return number & !(0x01 << bit);
}

pub fn set_bit(number: u32, bit: u8) -> u32 {
    return number | (0x01 << bit);
}

// A structure defining a register
// used in peripherals
pub struct Reg {
    base: u32,
    mask: u32,
}


