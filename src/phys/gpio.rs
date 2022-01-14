use crate::phys::addrs;
use crate::phys::*;

pub enum MuxSpeed {
    Slow,
    Fast,
}

pub enum Pin {
    Gpio1 = 1,
    Gpio2 = 2,
    Gpio3 = 3,
    Gpio4 = 4,
    Gpio5 = 5,
    Gpio6 = 6,
    Gpio7 = 7,
    Gpio8 = 8,
    Gpio9 = 9,
}

pub fn gpio_start_clock() {
    assign_bit(addrs::CCM_CCGR0, Bitwise::Or, 0x3 << 30);
    assign_bit(addrs::CCM_CCGR1, Bitwise::Or, (0x3 << 30) | (0x3 << 26));
    assign_bit(addrs::CCM_CCGR2, Bitwise::Or, 0x3 << 26);
    assign_bit(addrs::CCM_CCGR3, Bitwise::Or, 0x3 << 12);
}

fn get_addr(pin: &Pin) -> u32 {
    return match pin {
        Pin::Gpio1 => addrs::GPIO1,
        Pin::Gpio2 => addrs::GPIO2,
        Pin::Gpio3 => addrs::GPIO3,
        Pin::Gpio4 => addrs::GPIO4,
        Pin::Gpio5 => addrs::GPIO5,
        Pin::Gpio6 => addrs::GPIO6,
        Pin::Gpio7 => addrs::GPIO7,
        Pin::Gpio8 => addrs::GPIO8,
        Pin::Gpio9 => addrs::GPIO9,
    }
}

pub fn gpio_speed(pin: &Pin, speed: MuxSpeed) {

    // Gpio5 cannot be muxed.
    if match pin {
        Pin::Gpio5 => true,
        _ => false,
    } {
        return;
    }

    let addr = match pin {
        Pin::Gpio1 => addrs::IOMUXC_GPR_GPR26,
        Pin::Gpio6 => addrs::IOMUXC_GPR_GPR26,
        Pin::Gpio2 => addrs::IOMUXC_GPR_GPR27,
        Pin::Gpio7 => addrs::IOMUXC_GPR_GPR27,
        Pin::Gpio3 => addrs::IOMUXC_GPR_GPR28,
        Pin::Gpio8 => addrs::IOMUXC_GPR_GPR28,
        Pin::Gpio4 => addrs::IOMUXC_GPR_GPR29,
        Pin::Gpio9 => addrs::IOMUXC_GPR_GPR29,

        // This can't ever happen because Gpio5 is escape-hatched already
        _ => addrs::IOMUXC_GPR_GPR26,
    };

    match speed {
        MuxSpeed::Slow => {
            assign(addr, 0x0);
        },
        MuxSpeed::Fast => {
            assign(addr, 0xFFFF_FFFF);
        }
    }
}

pub fn gpio_direction(pin: &Pin, pad: u32, direction: Dir) {
    let addr = get_addr(pin) + 0x4;
    let original_value = read_word(addr);
    
    match direction {
        Dir::Input => {
            assign(addr, original_value & !(0x1 << pad));
        },
        Dir::Output => {
            assign(addr, original_value | (0x1 << pad));
        },
    };
}

pub fn gpio_set(pin: &Pin, mask: u32) {
    let addr = get_addr(pin) + 0x84;
    assign(addr, mask);
}

pub fn gpio_clear(pin: &Pin, mask: u32) {
    let addr = get_addr(pin) + 0x88;
    assign(addr, mask);
}

pub fn gpio_read(pin: &Pin, mask: u32) -> u32 {
    let addr = get_addr(pin) + 0x8;

    // Read from DSR first
    let word = read_word(get_addr(pin));

    return (read_word(addr) | word) & mask;
}