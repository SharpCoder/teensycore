#![allow(dead_code)]

use core::arch::asm;
use crate::phys::addrs;
use crate::phys::*;

const CTRL_BASE_REG: u32 = 0x18;
const DATA_BASE_REG: u32 = 0x1C;
const FIFO_BASE_REG: u32 = 0x28;
const WATERMARK_BASE_REG: u32 = 0x2C;

// Parity Type
pub const CTRL_PT: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1 };
// Parity Enable
pub const CTRL_PE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<1 };
// Idle Line Type Select
pub const CTRL_ILT: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<2 };
// 9-Bit or 8-Bit Mode Select
pub const CTRL_M: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<4 };
// Receiver Source Select 
pub const CTRL_RSRC: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<5 };
// Doze Enable
pub const CTRL_DOZEEN: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<6 };
// Loop Mode Select 
pub const CTRL_LOOPS: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<7 };
// Idle Configuration 
pub const CTRL_IDLECFG: Reg = Reg { base: CTRL_BASE_REG, mask: 0x7<<8 };
// 7-Bit Mode Select
pub const CTRL_M7: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<11 };
// Send Break 
pub const CTRL_SBK: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<16 };
// Receiver Enabled 
pub const CTRL_RE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<18 };
// Transmitter Enabled 
pub const CTRL_TE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<19 };
// Idle Line Interrupt Enabled
pub const CTRL_ILIE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<20 };
// Receiver Interrupt Enabled 
pub const CTRL_RIE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<21 };
// Transmission Complete Interrupt Enabled
pub const CTRL_TCIE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<22 };
// Transmit Interrupt Enabled
pub const CTRL_TIE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<23 };
// Receive FIFO Enable
pub const FIFO_RXFE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<3 };
// Transmit FIFO Enable
pub const FIFO_TXFE: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<7 };
// Receive FIFO Flush
pub const FIFO_RXFLUSH: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<14 };
// Transmit FIFO Flush
pub const FIFO_TXFLUSH: Reg = Reg { base: CTRL_BASE_REG, mask: 0x1<<15 };

// Input trigger mode (Controlled by XBAR, usually)
pub enum InputTrigger {
    Disabled,
    Rxd, // Input trigger modulates RXD
    CtsB, // Input trigger controls Clear-To-Send
    Txd, // Input trigger modulates TXD
}

pub enum Baud {
    Rate300 = 300,
    Rate2400 = 2400,
    Rate9600 = 9600,
}

pub enum ParityType {
    Even,
    Odd,
}

pub enum BitMode {
    NineBits,
    EightBits,
}

#[derive(Clone, Copy)]
pub enum IdleConfiguration {
    Idle1Char = 0x0,
    Idle2Char = 0x1,
    Idle4Char = 0x2,
    Idle8Char = 0x3,
    Idle16Char = 0x4,
    Idle32Char = 0x5,
    Idle64Char = 0x6,
    Idle128Char = 0x7,
}

#[derive(Copy, Clone)]
pub enum Device {
    Uart1,
    Uart2,
    Uart3,
    Uart4,
    Uart5,
    Uart6,
    Uart7,
    Uart8,
}

pub struct FifoConfig {
    pub tx_fifo_underflow_flag: bool,
    pub rx_fifo_underflow_flag: bool,
    pub tx_flush: bool,
    pub rx_flush: bool,
    // Receiver idle empty not supported currently
    pub tx_fifo_overflow_irq_en: bool,
    pub rx_fifo_underflow_irq_en: bool,
    pub tx_fifo_en: bool,
    pub rx_fifo_en: bool,
}

pub fn uart_clear_idle(device: Device) {
    let addr = get_addr(device) + 0x14;
    assign(addr, read_word(addr) | (0x1 << 20));
}

pub fn uart_or_reg(device: Device, register: &Reg, value: u32) {
    let addr = get_addr(device) + register.base;
    let val = read_word(addr) | value;
    assign(addr, val);
}

pub fn uart_and_reg(device: Device, register: &Reg, value: u32) {
    let addr = get_addr(device) + register.base;
    let val = read_word(addr) & value;
    assign(addr, val);
}

pub fn uart_set_reg(device: Device, register: &Reg) {
    let addr = get_addr(device) + register.base;
    let val = read_word(addr) | register.mask;
    assign(addr, val);
}

pub fn uart_clear_reg(device: Device, register: &Reg) {
    let addr = get_addr(device) + register.base;
    let val = read_word(addr) & !register.mask;
    assign(addr, val);
}

pub fn uart_invert_tx(device: Device, inverted: bool) {
    let addr = get_addr(device) + 0x18;
    let original = read_word(addr) ;
    let val = match inverted {
        true => original | 0x1 << 28,
        false => original & !(0x1 << 28),
    };

    assign(addr, val);
}

fn fifo_config_to_u32(config: &FifoConfig, baseline: u32) -> u32 {
    let mut result: u32 = baseline;   
    // Clear The rx_fifo_depth
    
    // Read Only
    // result = result & !0x7;
    // result = result & (config.rx_fifo_depth as u32);

    result = set_bit_from_bool(result, 3, config.rx_fifo_en);

    // Clear the tx_fifo_depth
    // Read Only
    // result = result & !(0x7 << 4);
    // result = result & (config.tx_fifo_depth as u32) << 4;

    result = set_bit_from_bool(result, 7, config.tx_fifo_en);
    result = set_bit_from_bool(result, 8, config.rx_fifo_underflow_irq_en);
    result = set_bit_from_bool(result, 9, config.tx_fifo_overflow_irq_en);
    result = set_bit_from_bool(result, 14, config.rx_flush);
    result = set_bit_from_bool(result, 15, config.tx_flush);
    result = set_bit_from_bool(result, 16, config.rx_fifo_underflow_flag);
    result = set_bit_from_bool(result, 17, config.tx_fifo_underflow_flag);
    return result;
}

pub struct UartConfig {
    // R8T9 not supported
    // R9T8 not supported
    // TXDIR not supported currently
    pub r9t8: bool,
    pub invert_transmission_polarity: bool,
    pub overrun_irq_en: bool,
    pub noise_error_irq_en: bool,
    pub framing_error_irq_en: bool,
    pub parity_error_irq_en: bool,
    pub tx_irq_en: bool,
    pub tx_complete_irq_en: bool,
    pub rx_irq_en: bool,
    pub idle_line_irq_en: bool,
    pub tx_en: bool,
    pub rx_en: bool,
    // Receiver wakeup control not supported
    // SBK not currently supported
    pub match1_irq_en: bool,
    pub match2_irq_en: bool,
    // 7-bit mode not supported
    pub idle_config: IdleConfiguration,
    // Loops not supported
    pub doze_en: bool,
    // RSRC not supported
    pub bit_mode: BitMode,
    // Received wakeup not supported
    // Line idle type not supported
    pub parity_en: bool,
    pub parity_type: ParityType,
}

fn set_bit_from_bool_without_clear(baseline: u32, bit: u8, value: bool) -> u32 {
    if value {
        return set_bit(baseline, bit);
    } else {
        return baseline;
    }
}

fn set_bit_from_bool(baseline: u32, bit: u8, value: bool) -> u32 {
    if value {
        return set_bit(baseline, bit);
    } else {
        return clear_bit(baseline, bit);
    }
}

fn config_to_u32(config: &UartConfig, baseline: u32) -> u32 {
    let mut result: u32 = baseline;
    
    match config.parity_type {
        ParityType::Even => {
            result = clear_bit(result, 0);
        },
        ParityType::Odd => {
            result = set_bit(result, 0);
        }
    }
    
    result = set_bit_from_bool(result, 1, config.parity_en);

    match config.bit_mode {
        BitMode::NineBits => {
            result = set_bit(result, 4);
        },
        BitMode::EightBits => {
            result = clear_bit(result, 4);
        }
    }

    result = set_bit_from_bool(result, 6, config.doze_en);

    // Clear idle config from original result
    result = result & !(0x7 << 8);
    result = result | (config.idle_config as u32) << 8;

    result = set_bit_from_bool(result, 14, config.match2_irq_en);
    result = set_bit_from_bool(result, 15, config.match1_irq_en);
    result = set_bit_from_bool(result, 18, config.rx_en);
    result = set_bit_from_bool(result, 19, config.tx_en);
    result = set_bit_from_bool(result, 20, config.idle_line_irq_en);
    result = set_bit_from_bool(result, 21, config.rx_irq_en);
    result = set_bit_from_bool(result, 22, config.tx_complete_irq_en);
    result = set_bit_from_bool(result, 23, config.tx_irq_en);
    result = set_bit_from_bool(result, 24, config.parity_error_irq_en);
    result = set_bit_from_bool(result, 25, config.framing_error_irq_en);
    result = set_bit_from_bool(result, 26, config.noise_error_irq_en);
    result = set_bit_from_bool(result, 27, config.overrun_irq_en);
    result = set_bit_from_bool(result, 28, config.invert_transmission_polarity);
    result = set_bit_from_bool(result, 30, config.r9t8);

    return result;
}

pub fn uart_start_clock() {
    // First, select the oscillator clock so all the math works
    assign(0x400F_C024, read_word(0x400F_C024) & !0x1F & !(0x1 << 6));

    assign(0x400FC07C, read_word(0x400FC07C) | (0x3 << 24));
    assign(0x400F_C074, read_word(0x400F_C074) | (0x3 << 2) | (0x3 << 6));
    assign(0x400F_C06C, read_word(0x400F_C06C) | (0x3 << 24));
    assign(0x400F_C068, read_word(0x400F_C068) | (0x3 << 12) | (0x3 << 28));
    assign(0x400F_C07C, read_word(0x400F_C07C) | (0x3 << 26));
}

pub fn get_addr(device: Device) -> u32 {
    return match device {
        Device::Uart1 => addrs::UART1,
        Device::Uart2 => addrs::UART2,
        Device::Uart3 => addrs::UART3,
        Device::Uart4 => addrs::UART4,
        Device::Uart5 => addrs::UART5,
        Device::Uart6 => addrs::UART6,
        Device::Uart7 => addrs::UART7,
        Device::Uart8 => addrs::UART8,
    };
}

// Set the software reset pin on or off
pub fn uart_sw_reset(device: Device, sw_reset: bool) {
    let value = match sw_reset {
        true => 0x2,
        false => 0x0,
    };

    assign(get_addr(device) + 0x8, value);

    // Solves what I believe is a timing issue.
    unsafe { asm!("nop"); }
}

pub fn uart_configure(device: Device, configuration: UartConfig) {
    let addr = get_addr(device) + 0x18;
    assign(addr, config_to_u32(&configuration, 0x0));
}

pub fn uart_set_tie(device: Device, en: bool) {
    let addr = get_addr(device) + 0x18;
    let origin = read_word(addr);

    let val = match en {
        true => origin | CTRL_TIE.mask,
        false => origin & !CTRL_TIE.mask,
    };

    assign(addr, val);
}

pub fn uart_configure_fifo(device: Device, configuration: FifoConfig) {
    let addr = get_addr(device) + 0x28;
    assign(addr, fifo_config_to_u32(&configuration, 0x0));
}

pub fn uart_set_pin_config(device: Device, mode: InputTrigger) {
    let addr = get_addr(device) + 0xC;
    match mode {
        InputTrigger::Disabled => { 
            assign(addr, 0x00);
        },
        InputTrigger::Rxd => { 
            assign(addr, 0x01);
        },
        InputTrigger::Txd => { 
            assign(addr, 0x03);
        },
        InputTrigger::CtsB => { 
            assign(addr, 0x02);
        },
    }
}

pub fn uart_enable(device: Device) {
    let addr = get_addr(device) + 0x18;
    let baseline = read_word(addr);
    assign(addr, baseline | (0x1 << 19) | (0x1 << 18));
    unsafe { asm!("nop"); }
}

pub fn uart_disable(device: Device) {
    let addr = get_addr(device) + 0x18;
    let baseline = read_word(addr);
    assign(addr, baseline & !((0x1 << 19) | (0x1 << 18)));
    unsafe { asm!("nop"); }
}

pub fn uart_write_fifo(device: Device, byte: u8) {
    let addr = get_addr(device) + 0x1C;
    assign_8(addr, byte as u8);
}

pub fn uart_queue_preamble(device: Device) {
    uart_write_fifo(device, 0x00);
}

pub fn uart_read_fifo(device: Device) -> u8 {
    let addr = get_addr(device) + 0x1c;
    return (read_word(addr) & 0x3ff) as u8;
}

/// Returns the depth of the transmit buffer
pub fn uart_get_tx_size(device: Device) -> u32 {
    let addr = get_addr(device) + 0x28;
    let config = read_word(addr) & 0x7;
    return match config {
        0x0 => 1,
        0x1 => 4,
        0x2 => 8,
        0x3 => 16,
        0x4 => 32,
        0x5 => 64,
        0x6 => 128,
        0x7 => 256,
        _ => 4,
    };
}

/// Returns how many bytes are in the tx fifo
pub fn uart_get_tx_count(device: Device) -> u32 {
    let addr = get_addr(device) + 0x2C;
    return (read_word(addr) & 0x700) >> 8;
}

pub fn uart_get_receive_count(device: Device) -> u32 {
    let addr = get_addr(device) + 0x2C;
    return (read_word(addr) & 7000000) >> 24;
}

pub fn uart_has_data(device: Device) -> bool {
    let addr = get_addr(device) + 0x1C;
    return (read_word(addr) & (0x1 << 12)) == 0;
}

pub fn uart_baud_rate(device: Device, rate: u32) {
    // TODO: Explain why this works (if it works)
    let baud_clock = 80000000; // MHz
    
    let sbr = baud_clock / (rate * 16);
    uart_disable(device);
    let addr = get_addr(device) + 0x10;
    let value = (read_word(addr) & !(0x1 << 13) & !(0x1FFF)) | (0x00 << 24) | (0x1 << 14) | (0x1 << 17)  | (0x1 << 18) | sbr;
    assign(addr, value);
    uart_enable(device);
}

pub fn uart_enable_dma(device: Device) {
    let addr = get_addr(device) + 0x10;
    assign(addr, read_word(addr) | (0x1 << 21) | (0x1 << 23));
}

pub fn uart_disable_dma(device: Device) {
    let addr = get_addr(device) + 0x10;
    assign(addr, read_word(addr) & !(0x1 << 21) & !(0x1 << 23));
}

pub fn uart_flush(device: Device) {
    let addr = get_addr(device) + 0x1C;
    let original = read_word(addr);
    assign(addr, original | (0x1<<15));
}

pub fn uart_sbk(device: Device) {
    let addr = get_addr(device) + 0x1C;
    let original = read_word(addr);
    assign(addr, original & !(0xFF) | (0x1 << 13));
}

pub fn uart_watermark(device: Device, val: u32) {
    let addr = get_addr(device) + 0x2C;
    assign(addr, (val & 0x3) | ((val & 0x3) << 16));
}

pub fn uart_enable_fifo(device: Device) {
    let addr = get_addr(device) + 0x28;
    assign(addr, read_word(addr) | (0x1 << 7));
}

pub fn uart_disable_fifo(device: Device) {
    let addr = get_addr(device) + 0x28;
    assign(addr, read_word(addr) & !(0x1 << 7));
}

pub fn uart_get_irq_statuses(device: Device) -> u32 {
    return read_word(get_addr(device) + 0x14);
}

pub struct UartClearIrqConfig {
    pub rx_overrun: bool,
    pub rx_idle: bool,
    pub rx_data_full: bool,
    pub rx_line_break: bool,
    pub rx_pin_active: bool,
    pub rx_set_data_inverted: bool, // This is not an irq, but it lives in the irq register
    pub tx_complete: bool,
    pub tx_empty: bool,
}

pub fn uart_clear_irq(device: Device, config: UartClearIrqConfig) {
    let addr = get_addr(device) + 0x14;
    let mut baseline = read_word(addr);

    baseline = set_bit_from_bool_without_clear(baseline, 31, config.rx_line_break);
    baseline = set_bit_from_bool_without_clear(baseline, 30, config.rx_pin_active);
    baseline = set_bit_from_bool_without_clear(baseline, 28, config.rx_set_data_inverted);
    baseline = set_bit_from_bool_without_clear(baseline, 23, config.tx_empty);
    baseline = set_bit_from_bool_without_clear(baseline, 22, config.tx_complete);
    baseline = set_bit_from_bool_without_clear(baseline, 21, config.rx_data_full);
    baseline = set_bit_from_bool_without_clear(baseline, 20, config.rx_idle);
    baseline = set_bit_from_bool_without_clear(baseline, 19, config.rx_overrun);

    assign(addr, baseline);
}