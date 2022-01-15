//! This module provides access to controlling discreet pins
//! over gpio.
//! 
//! Typically you need to configure the pin first, and then
//! interact with it. The following example will configure
//! pin 13 as an output and provide power to it:
//! 
//! ```no_run
//! use teensycore::phys::pins::*;
//! 
//! pin_mode(13, Mode::Output);
//! pin_out(13, Power::High);
//! ```
use crate::phys::addrs;
use crate::phys::*;
use crate::phys::gpio::*;

/// The mode indicating whether a pin is an Input or an Output
pub enum Mode {
    Output,
    Input,
}

/// The signal used to drive the pin. If it is HIGH the pin
/// will receive power. If it is low, the pin will be grounded.
pub enum Power {
    High,
    Low,
}

/// The gpio pad mux configuration. 
/// 
/// On the IMXRT, each gpio slot can be reconfigured to point
/// to a different peripheral. This enum defines which peripheral
/// is active. Follow the IMXRT documentation for specifics
/// on which alt setting maps to which pad.
pub enum Alt {
    Alt0 = 0x0,
    Alt1 = 0x1,
    Alt2 = 0x2,
    Alt3 = 0x3,
    Alt4 = 0x4,
    Alt5 = 0x5,
}

/// Whether the pin will have a pull-down resistor or 
/// a pull-up resistor.
pub enum PullUpDown {
    PullDown100k = 0x00,
    PullUp47k = 0x01,
    PullUp100k = 0x02,
    PullUp22k = 0x03,
}

pub enum PinSpeed {
    Low50MHz = 0x00,
    Medium100MHz = 0x01,
    Fast150MHz = 0x02,
    Max200MHz = 0x03,
}

pub enum PullKeep {
    Keeper = 0x00,
    Pull = 0x01,
}

pub enum DriveStrength {
    Disabled = 0x00,
    Max = 0x01,
    MaxDiv2 = 0x02,
    MaxDiv3 = 0x03,
    MaxDiv4 = 0x04,
    MaxDiv5 = 0x05,
    MaxDiv6 = 0x06,
    MaxDiv7 = 0x07,
}

pub struct PadConfig {
    pub hysterisis: bool,               // HYS
    pub resistance: PullUpDown,         // PUS
    pub pull_keep: PullKeep,            // PUE
    pub pull_keep_en: bool,             // PKE
    pub open_drain: bool,               // ODE
    pub speed: PinSpeed,                // SPEED
    pub drive_strength: DriveStrength,  // DSE
    pub fast_slew_rate: bool,           // SRE
}

/** The index is an arduino pin, the output is the teensy 4.0 bit */
const PIN_BITS: [u8; 40] = [
    3, 2, 4, 5, 6, 8, 10, 17, 
    16, 11, 0, 2, 1, 3, 18, 19, 
    23, 22, 17, 16, 26, 27, 24, 25, 
    12, 13, 30, 31, 18, 31, 23, 
    22, 12, 7, 15, 14, 13, 12, 17, 16,
];

/** The index is an arduino pin, the output is the gpio pin that controls it */
const PIN_TO_GPIO_PIN: [Pin; 40] = [
    Pin::Gpio6, Pin::Gpio6, Pin::Gpio9, Pin::Gpio9, Pin::Gpio9, Pin::Gpio9, Pin::Gpio7, Pin::Gpio7,
    Pin::Gpio7, Pin::Gpio7, Pin::Gpio7, Pin::Gpio7, Pin::Gpio7, Pin::Gpio7, Pin::Gpio6, Pin::Gpio6,
    Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6,
    Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio6, Pin::Gpio8, Pin::Gpio9, Pin::Gpio8, Pin::Gpio8,
    Pin::Gpio7, Pin::Gpio9, Pin::Gpio8, Pin::Gpio8, Pin::Gpio8, Pin::Gpio8, Pin::Gpio8, Pin::Gpio8,
];

/** The index is an arduino pin, the output is the IOMUX register which controls it */
const PIN_MUX: [u32;  40] = [
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B0_03, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B0_02,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_04, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_05,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_06, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_08,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_10, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B1_01,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B1_00, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_11,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_00, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_02,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_01, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_03,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_02, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_03,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_07, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_06,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_01, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_00,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_10, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_11,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_08, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_09,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B0_12, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B0_13,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_14, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_AD_B1_15,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_32, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_31,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_37, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_36,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_B0_12, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_EMC_07,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_03, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_02,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_01, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_00,
    addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_05, addrs::IOMUXC_SW_MUX_CTL_PAD_GPIO_SD_B0_04,
];


/// Reconfigure the pad which a particular gpio pin is
/// using.
pub fn pin_mux_config(pin: usize, alt: Alt) {
    let addr = PIN_MUX[pin];
    assign(addr, (read_word(addr) & !0x7) | alt as u32);
}

/// Configure all aspects of the pad.
/// 
/// This includes the speed, the resistance, the drive strength,
/// enabling hysterisis, and more.
pub fn pin_pad_config(pin: usize, config: PadConfig) {
    // -0x1F0 appears to universally be the difference
    // between the MUX_CTRL_PAD and the PAD_CTRL_PAD
    let addr = PIN_MUX[pin] - 0x1F0;
    let mut value = 0x0;

    value = value | ((0x1 & config.fast_slew_rate as u32) << 0);
    value = value | ((config.drive_strength as u32) << 3);
    value = value | ((config.speed as u32) << 6);
    value = value | ((0x1 & config.open_drain as u32) << 11);
    value = value | ((config.pull_keep_en as u32) << 12);
    value = value | ((config.pull_keep as u32) << 13);
    value = value | ((config.resistance as u32) << 14);
    value = value | ((0x1 & config.hysterisis as u32) << 16);

    assign(addr, value);
}

/// This method will configure the pin as an input or an output
pub fn pin_mode(pin: usize, mode: Mode) {
    gpio_speed(&PIN_TO_GPIO_PIN[pin], MuxSpeed::Fast);
    // gpio_clear(&PIN_TO_GPIO_PIN[pin], 0x1 << PIN_BITS[pin]);
    // Mux control pad

    match mode {
        Mode::Output => {
            // Make sure the pad is not overridden to be input
            // assign(PIN_MUX[pin], read_word(PIN_MUX[pin]) & !(0x1 << 4));
            gpio_direction(&PIN_TO_GPIO_PIN[pin], PIN_BITS[pin] as u32, Dir::Output);
        },
        Mode::Input => {
            // Mux the pad so it is overridden to be input
            // assign(PIN_MUX[pin], read_word(PIN_MUX[pin]) | (0x1 << 4));
            gpio_direction(&PIN_TO_GPIO_PIN[pin], PIN_BITS[pin] as u32, Dir::Input);
        }
    }
}

/// This method will output a high or low signal to the pin
pub fn pin_out(pin: usize, power: Power) {
    let mask = 0x1 << PIN_BITS[pin];
    match power {
        Power::High => {
            gpio_set(&PIN_TO_GPIO_PIN[pin], mask);
        },
        Power::Low => {
            gpio_clear(&PIN_TO_GPIO_PIN[pin], mask);
        }
    }
}

/// This method is a digital read of the specific pin
pub fn pin_read(pin: usize) -> u32 {
    let mask = 0x1 << PIN_BITS[pin];
    return gpio_read(&PIN_TO_GPIO_PIN[pin], mask);
}