//! This module represents the serial communication protocol
//! based on UART physical hardware.
//!
//! Typical operations in the kernel do not require direct uart access.
//! Instead, the `serio` interface has been devised which abstracts
//! much of the nuance away.
//!
//! On the Teensy4.0, Uart6 is what most would think of as the "Primary" uart.
//! It is located on pins 0 and 1.
//!
//! It is worth noting that the debug module of this kernel leverages
//! SerioDevice::Uart4 to output any debug data.
//!
//! Simple usage
//!
//! ```no_run
//! use teensycore::serio::*;
//!
//! serial_init(SerioDevice::Uart6);
//! serial_write(SerioDevice::Uart6, b"Hello, world!\r\n");
//!
//! while serial_available(SerioDevice::Uart6) > 0 {
//!     let sb = serial_read(SerioDevice::Uart6);
//!     // Do something with the Str
//! }
//! ```

#![allow(unused)]

use crate::debug::*;
use crate::phys::addrs;
use crate::phys::irq::*;
use crate::phys::pins::*;
use crate::phys::uart::*;
use crate::system::buffer::*;
use crate::system::str::*;
use crate::system::vector::*;

struct HardwareConfig {
    device: Device,
    tx_pin: usize,
    rx_pin: usize,
    irq: Irq,
    sel_inp_reg: Option<u32>,
    sel_inp_val: Option<u32>,
}

/// Enable this to mirror all bytes received
/// to the Debug UART peripheral.
const DEBUG_SPY: bool = false;
static mut TEMP_BUF: [u8; 128] = [0; 128];
const UART_WATERMARK_SIZE: u32 = 0x2;
const UART_BUFFER_DEPTH: usize = 512; // Note: this is repeated for every uart device. Don't make it too big.
static mut UART1: Uart = Uart::new(HardwareConfig {
    device: Device::Uart1,
    tx_pin: 24,
    rx_pin: 25,
    irq: Irq::Uart1,
    sel_inp_reg: None,
    sel_inp_val: None,
});

static mut UART2: Uart = Uart::new(HardwareConfig {
    device: Device::Uart2,
    tx_pin: 14,
    rx_pin: 15,
    irq: Irq::Uart2,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART2_RX_SELECT_INPUT),
    sel_inp_val: Some(0x1),
});

static mut UART3: Uart = Uart::new(HardwareConfig {
    device: Device::Uart3,
    tx_pin: 17,
    rx_pin: 16,
    irq: Irq::Uart3,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART3_RX_SELECT_INPUT),
    sel_inp_val: Some(0x0),
});

static mut UART4: Uart = Uart::new(HardwareConfig {
    device: Device::Uart4,
    tx_pin: 8,
    rx_pin: 7,
    irq: Irq::Uart4,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART4_RX_SELECT_INPUT),
    sel_inp_val: Some(0x2),
});

static mut UART5: Uart = Uart::new(HardwareConfig {
    device: Device::Uart5,
    tx_pin: 1,
    rx_pin: 0,
    irq: Irq::Uart5,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART5_RX_SELECT_INPUT),
    sel_inp_val: Some(0x0),
}); // NOTE: THIS DEVICE DOESN'T HAVE VALID PINS

static mut UART6: Uart = Uart::new(HardwareConfig {
    device: Device::Uart6,
    tx_pin: 1,
    rx_pin: 0,
    irq: Irq::Uart6,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART6_RX_SELECT_INPUT),
    sel_inp_val: Some(0x1),
});

static mut UART7: Uart = Uart::new(HardwareConfig {
    device: Device::Uart7,
    tx_pin: 29,
    rx_pin: 28,
    irq: Irq::Uart7,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART7_RX_SELECT_INPUT),
    sel_inp_val: Some(0x1),
});

static mut UART8: Uart = Uart::new(HardwareConfig {
    device: Device::Uart8,
    tx_pin: 20,
    rx_pin: 21,
    irq: Irq::Uart8,
    sel_inp_reg: Some(addrs::IOMUXC_LPUART8_RX_SELECT_INPUT),
    sel_inp_val: Some(0x0),
});

#[derive(Clone, Copy)]
pub enum SerioDevice {
    Uart1 = 0x0,
    Uart2 = 0x1,
    Uart3 = 0x2,
    Uart4 = 0x3,
    Uart5 = 0x4,
    Uart6 = 0x5,
    Uart7 = 0x6,
    Uart8 = 0x7,
    Default = 0x8,
    Debug = 0x9,
}

/**
    This encapsulates an entire Uart device
    being instantiated, including all necessary memory
    and mappings.
*/
struct Uart {
    device: Device,
    tx_pin: usize,
    rx_pin: usize,
    initialized: bool,
    irq: Irq,
    tx_buffer: Buffer<UART_BUFFER_DEPTH, u8>,
    rx_buffer: Str,
    sel_inp_reg: Option<u32>,
    sel_inp_val: Option<u32>,
    buffer_head: usize,
    tx_count: u32,
    paused: bool,
}

impl Uart {
    pub const fn new(config: HardwareConfig) -> Uart {
        return Uart {
            device: config.device,
            tx_buffer: Buffer {
                data: [0; UART_BUFFER_DEPTH],
                tail: 0,
            },
            rx_buffer: Str::new(),
            buffer_head: 0,
            initialized: false,
            tx_pin: config.tx_pin,
            rx_pin: config.rx_pin,
            sel_inp_reg: config.sel_inp_reg,
            sel_inp_val: config.sel_inp_val,
            irq: config.irq,
            tx_count: 0,
            paused: false,
        };
    }

    fn initialize(&mut self) {
        if self.initialized {
            return;
        }

        // Initialize the pins
        pin_mux_config(self.tx_pin, Alt::Alt2);
        pin_pad_config(
            self.tx_pin,
            PadConfig {
                hysterisis: true,
                resistance: PullUpDown::PullDown100k,
                pull_keep: PullKeep::Keeper,
                pull_keep_en: false,
                open_drain: false,
                speed: PinSpeed::Low50MHz,
                drive_strength: DriveStrength::MaxDiv3,
                fast_slew_rate: true,
            },
        );

        pin_mux_config(self.rx_pin, Alt::Alt2);
        pin_pad_config(
            self.rx_pin,
            PadConfig {
                hysterisis: true,
                resistance: PullUpDown::PullUp22k,
                pull_keep: PullKeep::Pull,
                pull_keep_en: true,
                open_drain: false,
                speed: PinSpeed::Low50MHz,
                drive_strength: DriveStrength::MaxDiv3,
                fast_slew_rate: false,
            },
        );

        // Configure the base settings
        uart_disable(self.device);
        uart_sw_reset(self.device, true);
        uart_sw_reset(self.device, false);
        uart_configure(
            self.device,
            UartConfig {
                r9t8: false,
                invert_transmission_polarity: false,
                overrun_irq_en: true,
                noise_error_irq_en: false,
                framing_error_irq_en: false,
                parity_error_irq_en: false,
                tx_irq_en: false, // This gets set later
                rx_irq_en: true,
                tx_complete_irq_en: true,
                idle_line_irq_en: true,
                tx_en: false,
                rx_en: false,
                match1_irq_en: false,
                match2_irq_en: false,
                idle_config: IdleConfiguration::Idle64Char,
                doze_en: false,
                bit_mode: BitMode::EightBits,
                parity_en: false,
                parity_type: ParityType::Even,
            },
        );

        uart_configure_fifo(
            self.device,
            FifoConfig {
                tx_fifo_underflow_flag: false,
                rx_fifo_underflow_flag: false,
                tx_flush: false,
                rx_flush: false,
                tx_fifo_overflow_irq_en: false,
                rx_fifo_underflow_irq_en: true,
                tx_fifo_en: true,
                rx_fifo_en: true,
            },
        );

        uart_set_pin_config(self.device, InputTrigger::Disabled);
        uart_disable_fifo(self.device);

        uart_watermark(self.device, UART_WATERMARK_SIZE);
        uart_enable(self.device);

        pin_mode(self.tx_pin, Mode::Output);
        pin_mode(self.rx_pin, Mode::Input);

        // If this uart requires additional input muxing, do it.
        if self.sel_inp_reg.is_some() {
            crate::phys::assign(self.sel_inp_reg.unwrap(), self.sel_inp_val.unwrap());
        }

        pin_out(self.tx_pin, Power::Low);

        irq_attach(self.irq, serio_handle_irq);
        irq_enable(self.irq);
        irq_priority(self.irq, 128);
        uart_baud_rate(self.device, 115200);

        self.initialized = true;
    }

    pub fn available(&self) -> usize {
        return self.rx_buffer.len();
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for byte_idx in 0..bytes.len() {
            self.tx_buffer.enqueue(bytes[byte_idx]);
        }

        uart_set_reg(self.device, &CTRL_TCIE);
        pin_out(self.tx_pin, Power::High);
    }

    pub fn write_vec(&mut self, bytes: &Vector<u8>) {
        for item in bytes.into_iter() {
            self.tx_buffer.push(item);
        }

        pin_out(self.tx_pin, Power::High);
        uart_set_reg(self.device, &CTRL_TCIE);
    }

    pub fn get_rx_buffer(&mut self) -> &mut Str {
        return &mut self.rx_buffer;
    }

    fn handle_receive_irq(&mut self) {
        let irq_statuses = uart_get_irq_statuses(self.device);

        // TODO: Implement some logic for these edge cases
        // but it's really not needed for just simply
        // receiving messages.
        let rx_overrun = irq_statuses & (0x1 << 19) > 0;
        // let rx_active = irq_statuses & (0x1 << 24) > 0;
        // let rx_buffer_full = irq_statuses & (0x1 << 21) > 0;
        // let rx_idle = irq_statuses & (0x1 << 20) > 0;

        // Read until it is empty
        let mut count = 0;
        while uart_has_data(self.device) {
            let msg: u8 = uart_read_fifo(self.device);
            self.rx_buffer.append(&[msg]);
            unsafe { TEMP_BUF[count] = msg };
            count += 1;
        }

        if DEBUG_SPY {
            for idx in 0..count {
                serial_write(SerioDevice::Debug, &[unsafe { TEMP_BUF[idx] }]);
            }
        }

        if rx_overrun {
            crate::debug::blink_accumulate();
        }
    }

    fn transmit(&mut self) {
        match self.tx_buffer.dequeue() {
            None => {}
            Some(byte) => {
                // Get the next byte to write and beam it
                uart_write_fifo(self.device, byte);
            }
        }
    }

    fn handle_send_irq(&mut self) {
        // Transmission complete
        let irq_statuses = uart_get_irq_statuses(self.device);
        let tx_complete = irq_statuses & (0x1 << 22) > 0;
        let pending_data = self.tx_buffer.size() > 0;

        // Check if there is space in the buffer
        if pending_data && tx_complete {
            self.transmit();
        } else if !pending_data {
            uart_clear_reg(self.device, &CTRL_TCIE);
        }
    }

    pub fn handle_irq(&mut self) {
        // Don't process a uart device that hasn't
        // been used
        if !self.initialized {
            return;
        }

        self.handle_receive_irq();
        self.handle_send_irq();
        uart_clear_irq(self.device);
    }
}

fn get_uart_interface(device: SerioDevice) -> &'static mut Uart {
    unsafe {
        return match device {
            SerioDevice::Uart1 => &mut UART1,
            SerioDevice::Uart2 => &mut UART2,
            SerioDevice::Uart3 => &mut UART3,
            SerioDevice::Uart4 => &mut UART4,
            SerioDevice::Uart5 => &mut UART5,
            SerioDevice::Uart6 => &mut UART6,
            SerioDevice::Uart7 => &mut UART7,
            SerioDevice::Uart8 => &mut UART8,

            // Specify debug output here
            SerioDevice::Debug => &mut UART4,

            // Specify defaut here
            SerioDevice::Default => &mut UART6,
        };
    }
}

/// Initializes the serial device. This configures
/// and muxes the relevant pins, sets baud rate,
/// enables peripheral device, and generally
/// wakes up the uart.
pub fn serial_init(device: SerioDevice) {
    let uart = get_uart_interface(device);
    uart.initialize();
}

/// Retuns the current buffer of data the serial interface
/// has accumulated.
///
/// You can interact with this buffer, modify it, etc. It is
/// recommended to call `.clear()` on the buffer as soon as you are
/// done with the data, otherwise the buffer will eventually
/// overflow.
pub fn serial_read<'a>(device: SerioDevice) -> &'a mut Str {
    let uart = get_uart_interface(device);
    return uart.get_rx_buffer();
}

/// Returns the amount of data in the currenet read buffer.
pub fn serial_available(device: SerioDevice) -> usize {
    let uart = get_uart_interface(device);
    return uart.available();
}

/// Enqueue data to be written over serial.
///
/// This data will be written at the next available interrupt
/// cycle.
pub fn serial_write(device: SerioDevice, bytes: &[u8]) {
    let uart = get_uart_interface(device);
    uart.write(bytes);
}

pub fn serial_write_vec(device: SerioDevice, bytes: &Vector<u8>) {
    let uart = get_uart_interface(device);
    for byte in bytes.into_iter() {
        uart.write(&[byte]);
    }
}

pub fn serial_write_str(device: SerioDevice, bytes: &mut Str) {
    let uart = get_uart_interface(device);
    for byte in bytes.into_iter() {
        uart.write(&[byte]);
    }

    // Fixes memory leak. When calling this function you'll
    // usually be operating with an intermediary string and
    // won't be able to drop() it yourself.
    bytes.drop();
}

pub fn serial_baud(device: SerioDevice, rate: u32) {
    let uart = get_uart_interface(device);
    uart_baud_rate(uart.device, rate);
}

pub fn serio_handle_irq() {
    irq_disable(Irq::Uart1);
    irq_disable(Irq::Uart2);
    irq_disable(Irq::Uart3);
    irq_disable(Irq::Uart4);
    irq_disable(Irq::Uart5);
    irq_disable(Irq::Uart6);
    irq_disable(Irq::Uart7);
    irq_disable(Irq::Uart8);

    get_uart_interface(SerioDevice::Uart1).handle_irq();
    get_uart_interface(SerioDevice::Uart2).handle_irq();
    get_uart_interface(SerioDevice::Uart3).handle_irq();
    get_uart_interface(SerioDevice::Uart4).handle_irq();
    get_uart_interface(SerioDevice::Uart5).handle_irq();
    get_uart_interface(SerioDevice::Uart6).handle_irq();

    irq_enable(Irq::Uart1);
    irq_enable(Irq::Uart2);
    irq_enable(Irq::Uart3);
    irq_enable(Irq::Uart4);
    irq_enable(Irq::Uart5);
    irq_enable(Irq::Uart6);
    irq_enable(Irq::Uart7);
    irq_enable(Irq::Uart8);
}
