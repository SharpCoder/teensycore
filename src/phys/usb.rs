#![allow(dead_code)]

mod descriptors;
pub mod models;
mod registers;

use descriptors::*;
use models::*;
use registers::*;

use crate::debug::*;
use crate::mem::zero;
use crate::phys::addrs::*;
use crate::phys::irq::*;
use crate::phys::read_word;
use crate::system::vector::Stack;
use crate::system::vector::Vector;
use crate::*;
use crate::{assembly, phys::*};

pub const USBINT: u32 = 1;
pub const USBERRINT: u32 = 2;
pub const PCI: u32 = 1 << 2;
pub const FRI: u32 = 1 << 3;
pub const SEI: u32 = 1 << 4;
pub const URI: u32 = 1 << 6;
pub const SRI: u32 = 1 << 7;
pub const SLI: u32 = 1 << 8;
pub const HCH: u32 = 1 << 12;
pub const NAKE: u32 = 1 << 16;
pub const TI0: u32 = 1 << 24;
pub const TI1: u32 = 1 << 25;

/** Registers */

/************************/

const BLANK_QUEUE_HEAD: UsbEndpointQueueHead = UsbEndpointQueueHead {
    config: 0,
    current: 0,
    next: 0,
    status: 0,
    pointer0: 0,
    pointer1: 0,
    pointer2: 0,
    pointer3: 0,
    pointer4: 0,
    reserved: 0,
    setup0: 0,
    setup1: 0,
    callback: noop,
};

#[no_mangle]
#[link_section = ".endpoint_queue"]
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; 16] = [BLANK_QUEUE_HEAD; 16];
#[no_mangle]
#[link_section = ".dmabuffers"]
static mut USB_DESCRIPTOR_BUFFER: [u8; 512] = [0; 512];
#[no_mangle]
static mut ENDPOINT0_TRANSFER_DATA: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor {
    next: 0,
    status: 0,
    pointer0: 0,
    pointer1: 0,
    pointer2: 0,
    pointer3: 0,
    pointer4: 0,
    callback: noop,
};
#[no_mangle]
static mut ENDPOINT0_TRANSFER_ACK: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor {
    next: 0,
    status: 0,
    pointer0: 0,
    pointer1: 0,
    pointer2: 0,
    pointer3: 0,
    pointer4: 0,
    callback: noop,
};

static mut CONFIGURATION_CALLBACKS: Vector<ConfigFn> = Vector::new();
static mut CONFIGURATION: u16 = 0;
static mut HIGHSPEED: bool = false;

/// This method will attach a callback to be invoked
/// when a setup packet is received. See usb_serial.rs
/// for examples.
pub fn usb_attach_setup_callback(callback: ConfigFn) {
    unsafe {
        CONFIGURATION_CALLBACKS.push(callback);
    }
}

pub fn usb_start_clock() {
    wait_exact_ns(MS_TO_NANO * 20);

    loop {
        let pll_usb1_status = read_word(PLL1_USB1_ADDR);
        if (pll_usb1_status & (0x1 << 13)) == 0 {
            // ENABLE
            assign(PLL1_USB1_ADDR_SET, 0x1 << 13); // SET
            continue;
        }

        if (pll_usb1_status & (0x1 << 12)) == 0 {
            // POWER
            assign(PLL1_USB1_ADDR_SET, 0x1 << 12); // SET
            continue;
        }

        if (pll_usb1_status & (0x1 << 16)) > 0 {
            // LOCK
            continue;
        }

        if (pll_usb1_status & (0x1 << 31)) == 0 {
            // BYPASS
            assign(PLL1_USB1_ADDR_CLR, 0x1 << 31); // CLEAR
            continue;
        }

        if (pll_usb1_status & (0x1 << 6)) == 0 {
            // CLEAR CLOCKGTE
            assign(0x400D_9038, 0x1 << 30);
            // USB_EN_CLK
            assign(PLL1_USB1_ADDR_SET, 0x1 << 6); // SET
            continue;
        }

        break;
    }

    assign(0x400D_8000 + 0x120, 0xF << 8 | 6 << 4 | 1);
    // Ungate the USB clock
    assign(0x400F_C000 + 0x80, read_word(0x400F_C000 + 0x80) | 0x1);
    // Set usb burstsize
    assign(USB + 0x160, 0x0404);
}

pub fn usb_initialize() {
    // Reset
    // *********************************
    assign(USBPHY1_CTRL_SET, 1 << 31);
    usb_cmd(read_word(USBCMD) | 2);
    while (read_word(USBCMD) & 0x2) > 0 {
        assembly!("nop");
    }

    irq_clear_pending();

    assign(USBPHY1_CTRL_CLR, 1 << 31);
    wait_exact_ns(MS_TO_NANO * 25);
    assign(USBPHY1_CTRL_CLR, 1 << 30);
    assign(USBPHY1_PWD, 0);

    // *********************************

    usb_set_mode(UsbMode::DEVICE);
    endpoint0_initialize();
    usb_irq_enable(0x143); // 0x143, 0x30105FF, | (1 << 24) | (1 << 25)

    usb_cmd(1); // Run/Stop bit

    debug_str(b"[usb] booting...");

    // Configure timer
    // assign(USB + 0x80, 0x0003E7); // 1ms
    // assign(USB + 0x84, (1 << 31) | (1 << 24) | 0x0003E7);
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            // Enter device mode and set the SLOM bit
            assign(USBMODE, 0x2 | (0x1 << 3));
        }
    }
}

/// Wait for a register to read all 0's across the borad
fn usb_waitfor(addr: u32) {
    while read_word(addr) > 0 {
        assembly!("nop");
    }
}

/// Enable all usb interrupts
pub fn usb_irq_enable(value: u32) {
    assign(USBINTR, value);

    irq_attach(Irq::Usb1, handle_usb_irq);
    irq_priority(Irq::Usb1, 32);
    irq_enable(Irq::Usb1);
}

/// Clear specific interrupts from the status field.
pub fn usb_irq_clear(value: u32) {
    assign(USBSTS, value);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb1);
    assign(USBINTR, 0x0);
}

/// Internal method to initialize the control endpoints
fn endpoint0_initialize() {
    unsafe {
        let epaddr = &ENDPOINT_HEADERS[0] as *const UsbEndpointQueueHead as u32;

        // 4096 bytes per the linker file
        zero(epaddr, 4096);

        // Priming the headers
        // First, set max_packet_size
        ENDPOINT_HEADERS[0].config |= (64 << 16) | (1 << 15); // RX
        ENDPOINT_HEADERS[1].config |= 64 << 16; // TX

        assign(ENDPTLISTADDR, epaddr);
    }
}

/// Assign a value to the command register.
pub fn usb_cmd(val: u32) {
    assign(USBCMD, val);
}

/// Return true if we are in highspeed mode.
pub fn usb_is_highspeed() -> bool {
    return unsafe { HIGHSPEED };
}

/// Helper method to configure an endpoint queuehead.
fn configure_ep(qh: &mut UsbEndpointQueueHead, config: u32, cb: Option<Fn>) {
    qh.config = config;
    qh.next = 1;

    if cb.is_some() {
        qh.callback = cb.unwrap();
    }
}

/// Use this method to fully configure an endpoint
pub fn usb_setup_endpoint(
    index: usize,
    tx_config: Option<EndpointConfig>,
    rx_config: Option<EndpointConfig>,
) {
    let tx_qh = unsafe { &mut ENDPOINT_HEADERS[index * 2 + 1] };
    let rx_qh = unsafe { &mut ENDPOINT_HEADERS[index * 2] };
    let ep_control_addr = USB + 0x1C0 + (index as u32) * 4;

    let isochornous = 1;
    let bulk = 2;
    let interrupt = 3;
    let tx_enable_bit = 1 << 23;
    let rx_enable_bit = 1 << 7;

    if tx_config.is_some() {
        let config = tx_config.unwrap();
        match config.endpoint_type {
            EndpointType::ISOCHRONOUS => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (isochornous << 18) | tx_enable_bit,
                );
            }
            EndpointType::BULK => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (bulk << 18) | tx_enable_bit,
                );
            }
            EndpointType::INTERRUPT => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (interrupt << 18) | tx_enable_bit,
                );
            }
        }

        configure_ep(tx_qh, config.size << 16, config.callback);
    }

    if rx_config.is_some() {
        let config = rx_config.unwrap();
        match config.endpoint_type {
            EndpointType::ISOCHRONOUS => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (isochornous << 2) | rx_enable_bit,
                );
            }
            EndpointType::BULK => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (bulk << 2) | rx_enable_bit,
                );
            }
            EndpointType::INTERRUPT => {
                assign(
                    ep_control_addr,
                    read_word(ep_control_addr) | (interrupt << 2) | rx_enable_bit,
                );
            }
        }

        configure_ep(rx_qh, config.size << 16, config.callback);
    }
}

fn usb_prime_endpoint(index: u32, tx: bool) {
    let mask = match tx {
        true => 1 << (16 + index),
        false => 1 << index,
    };

    assign(ENDPTPRIME, mask);

    loop {
        let prime = read_word(ENDPTPRIME) & mask;
        let stat = read_word(ENDPTSTAT) & mask;

        if prime == 0 && stat != 0 {
            // Success
            return;
        } else if prime == 0 && stat == 0 {
            // Failure
            debug_str(b"failed to prime");
            return;
        } else {
            assembly!("nop");
        }
    }
}

#[no_mangle]
#[inline]
fn endpoint0_setup(packet: SetupPacket) {
    for callback in unsafe { CONFIGURATION_CALLBACKS.into_iter() } {
        callback(packet);
    }

    match packet.bm_request_and_type {
        0x681 | 0x680 => {
            debug_str(b"GET_DESCRIPTOR");
            for descriptor in DESCRIPTOR_LIST {
                let dw_value = descriptor.w_value as u16;
                let dw_index = descriptor.w_index as u16;

                if dw_value == packet.w_value && dw_index == packet.w_index {
                    // Transmit the data
                    match descriptor.payload {
                        DescriptorPayload::Device(bytes) => {
                            debug_str(b"tx device descriptor");
                            endpoint0_transmit(&bytes, bytes.len(), false);
                        }
                        DescriptorPayload::Qualifier(bytes) => {
                            debug_str(b"tx qualifier descriptor");
                            endpoint0_transmit(&bytes, bytes.len(), false);
                        }
                        DescriptorPayload::Config(bytes) => {
                            debug_str(b"tx config descriptor");
                            // Send the correct variant
                            endpoint0_transmit(&bytes, bytes.len(), false);
                        }
                        DescriptorPayload::SupportedLanguages(language_codes) => {
                            debug_str(b"tx first string");

                            // Create some obscenely large buffer and
                            // build from that.
                            let mut bytes: [u8; 64] = [0; 64];
                            bytes[0] = (language_codes.len() * 2 + 2) as u8;
                            bytes[1] = 3;
                            for idx in 0..language_codes.len() {
                                bytes[idx * 2 + 2] = (language_codes[idx] & 0xFF) as u8; // lsb
                                bytes[idx * 2 + 3] = (language_codes[idx] >> 8) as u8;
                                // msb
                            }

                            endpoint0_transmit(&bytes, language_codes.len() * 2 + 2, false);
                        }
                        DescriptorPayload::String(characters) => {
                            debug_str(b"tx string");
                            let mut bytes: [u8; 64] = [0; 64];
                            bytes[0] = (2 * characters.len() + 2) as u8;
                            bytes[1] = 3;
                            for idx in 0..characters.len() {
                                bytes[idx * 2 + 2] = characters[idx]; // lsb
                                bytes[idx * 2 + 3] = 0x0; // msb
                            }

                            endpoint0_transmit(&bytes, 2 * characters.len() + 2, false);
                        }
                    }

                    return;
                }
            }

            debug_hex(packet.bm_request_and_type as u32, b"bm_request_and_type");
            debug_hex(packet.w_value as u32, b"w_value");
            debug_hex(packet.w_index as u32, b"w_index");
            debug_hex(packet.w_length as u32, b"w_length");
            debug_str(b"didn't find descriptor");
        }
        0x500 => {
            // Set Address
            endpoint0_receive(0, 0, false);
            assign(DEVICEADDR, ((packet.w_value as u32) << 25) | (1 << 24));
            debug_u64(packet.w_value as u64, b"SET_ADDRESS");
            return;
        }
        0x900 => {
            // Set configuration
            debug_str(b"SET_CONFIGURATION");
            unsafe {
                CONFIGURATION = packet.w_value;
            }
            endpoint0_receive(0, 0, false);
            return;
        }
        0x880 => {
            // Get configuration
            debug_str(b"GET_CONFIGURATION");
        }
        0x80 => {
            // Get status (device)
            debug_str(b"GET_STATUS (device)");
        }
        0x82 => {
            // Get status (endpoint)
            debug_str(b"GET_STATUS (endpoint)");
        }
        0x302 => {
            // Set feature
            debug_str(b"SET_FEATURE");
        }
        0x102 => {
            // Clear feature
            debug_str(b"CLEAR_FEATURE");
        }
        _ => {
            debug_str(b"UNKNOWN");
            debug_u64(packet.bm_request_and_type as u64, b"bm_request_and_type");
            debug_u64(packet.w_value as u64, b"w_value");
            debug_u64(packet.w_index as u64, b"w_index");
            debug_u64(packet.w_length as u64, b"w_length");
        }
    }

    assign(ENDPTCTRL0, (1 << 16) | 1); // Stall
}

fn endpoint0_transmit(bytes: &[u8], byte_length: usize, notify: bool) {
    // Do the transmit
    let len = byte_length as u32;
    let src_addr = bytes.as_ptr() as u32;
    let usb_descriptor_buffer_addr = unsafe { USB_DESCRIPTOR_BUFFER.as_ptr() } as u32;
    arm_dcache_delete(usb_descriptor_buffer_addr, len);
    mem::copy(src_addr, usb_descriptor_buffer_addr, len);

    if len > 0 {
        unsafe {
            ENDPOINT0_TRANSFER_DATA.next = 1;
            ENDPOINT0_TRANSFER_DATA.status = (len << 16) | (1 << 7);

            let addr = USB_DESCRIPTOR_BUFFER.as_ptr() as u32;
            let endpoint0_transfer_data_addr =
                &ENDPOINT0_TRANSFER_DATA as *const UsbEndpointTransferDescriptor as u32;

            ENDPOINT0_TRANSFER_DATA.pointer0 = addr;
            ENDPOINT0_TRANSFER_DATA.pointer1 = addr + 4096;
            ENDPOINT0_TRANSFER_DATA.pointer2 = addr + 8192;
            ENDPOINT0_TRANSFER_DATA.pointer3 = addr + 12288;
            ENDPOINT0_TRANSFER_DATA.pointer4 = addr + 16384;

            ENDPOINT_HEADERS[1].next = endpoint0_transfer_data_addr;
            ENDPOINT_HEADERS[1].status = 0;
        }

        usb_prime_endpoint(0, true);
    }

    unsafe {
        ENDPOINT0_TRANSFER_ACK.next = 1;
        match notify {
            true => {
                ENDPOINT0_TRANSFER_ACK.status = (1 << 7) | (1 << 15);
            }
            false => {
                ENDPOINT0_TRANSFER_ACK.status = 1 << 7;
            }
        }
        ENDPOINT0_TRANSFER_ACK.pointer0 = 0;
        ENDPOINT_HEADERS[0].next =
            (&ENDPOINT0_TRANSFER_ACK as *const UsbEndpointTransferDescriptor) as u32;
        ENDPOINT_HEADERS[0].status = 0;
    }

    usb_prime_endpoint(0, false);
}

fn endpoint0_receive(addr: u32, len: u32, notify: bool) {
    if len > 0 {
        unsafe {
            ENDPOINT0_TRANSFER_DATA.next = 1;
            ENDPOINT0_TRANSFER_DATA.status = (len << 16) | (1 << 7);
            ENDPOINT0_TRANSFER_DATA.pointer0 = addr;
            ENDPOINT0_TRANSFER_DATA.pointer1 = addr + 4096;
            ENDPOINT0_TRANSFER_DATA.pointer2 = addr + 8192;
            ENDPOINT0_TRANSFER_DATA.pointer3 = addr + 12288;
            ENDPOINT0_TRANSFER_DATA.pointer4 = addr + 16384;
            let endpoint0_transfer_data_addr =
                (&ENDPOINT0_TRANSFER_DATA as *const UsbEndpointTransferDescriptor) as u32;

            ENDPOINT_HEADERS[0].next = endpoint0_transfer_data_addr;
            ENDPOINT_HEADERS[0].status = 0;
        }

        usb_prime_endpoint(0, false);
    }

    unsafe {
        ENDPOINT0_TRANSFER_ACK.next = 1;
        match notify {
            true => {
                ENDPOINT0_TRANSFER_ACK.status = (1 << 7) | (1 << 15);
            }
            false => {
                ENDPOINT0_TRANSFER_ACK.status = 1 << 7;
            }
        }
        ENDPOINT0_TRANSFER_ACK.pointer0 = 0;
        ENDPOINT_HEADERS[1].next =
            (&ENDPOINT0_TRANSFER_ACK as *const UsbEndpointTransferDescriptor) as u32;
        ENDPOINT_HEADERS[1].status = 0;
    }

    assign(ENDPTCOMPLETE, 0xFFFF_FFFF);
    usb_prime_endpoint(0, true);
}

fn handle_usb_irq() {
    let show_messages = false;
    let irq_status = read_word(USBSTS);
    assembly!("nop");
    usb_irq_clear(irq_status);

    // debug_str(b"[usb] irq begin");
    if (irq_status & HCH) > 0 {
        debug_str(b"[usb] DCHalted!!!!!!!!!");
    }

    if (irq_status & PCI) > 0 {
        if show_messages {
            debug_str(b" -> [usb] Port change");
        }

        // Check which mode we are in
        let port_status = read_word(PORTSC1);
        if port_status & (0x1 << 9) > 0 {
            unsafe {
                HIGHSPEED = true;
            }
        } else {
            unsafe {
                HIGHSPEED = false;
            }
        }

        if (port_status & 1) > 0 {
            // Attached
            // debug_str(b"attached");
        }
    }

    if (irq_status & SEI) > 0 {
        debug_str(b"[usb] system error flag detected");
    }

    if (irq_status & NAKE) > 0 {
        debug_str(b" -> [usb] NAK flag detected");
    }

    if (irq_status & SRI) > 0 {
        //     if show_messages {
        //         debug_str(b" -> [usb] Start of Frame flag detected");
        //     }
    }

    if (irq_status & USBERRINT) > 0 {
        debug_str(b" -> [usb] USBERRINT flag detected");
    }

    if (irq_status & SLI) > 0 {
        debug_str(b" -> [usb] suspend");
    }

    if (irq_status & URI) > 0 {
        debug_str(b" -> [usb] URI flag detected");
        assign(ENDPTSTAT, read_word(ENDPTSTAT));
        assign(ENDPTCOMPLETE, read_word(ENDPTCOMPLETE));

        // Wait for endpoint priming to finish
        usb_waitfor(ENDPTPRIME);

        // Flush all endpoints
        assign(ENDPTFLUSH, 0xFFFF_FFFF);

        // Read the reset bit and make sure it is still active
        let port_status = read_word(PORTSC1);
        if (port_status & (1 << 8)) == 0 {
            debug_str(b"[usb] ERROR PORT STATUS");
        } else {
            // Still active
        }
    }

    if (irq_status & USBINT) > 0 {
        // debug_str(b" -> [usb] USBINT flag detected");
        let mut setup_status = read_word(ENDPTSETUPSTAT);
        while setup_status > 0 {
            // Clear the setup status
            assign(ENDPTSETUPSTAT, setup_status);

            // Duplicate setup buffer into local byte array
            let mut setup_packet;

            loop {
                // Set tripwire
                assign(USBCMD, read_word(USBCMD) | (1 << 13));

                // Copy the setup queue
                unsafe {
                    setup_packet = SetupPacket::from_dwords(
                        ENDPOINT_HEADERS[0].setup0,
                        ENDPOINT_HEADERS[0].setup1,
                    );
                }

                // Check for finish condition
                if (read_word(USBCMD) & (1 << 13)) > 0 {
                    break;
                }
            }

            // Write 0 to clear the tripwire
            assign(USBCMD, read_word(USBCMD) & !(1 << 13));

            // Flush endpoint
            assign(ENDPTFLUSH, (1 << 16) | 1);

            // Wait for the flush to finish
            usb_waitfor(ENDPTFLUSH);

            if (read_word(ENDPTSTAT) & ((1 << 16) | 1)) > 0 {
                debug_str(b"Endpoint Flush FAILED");
            }

            // Setup
            endpoint0_setup(setup_packet);

            // Check for another packet
            setup_status = read_word(ENDPTSETUPSTAT);
        }

        let complete_status = read_word(ENDPTCOMPLETE);
        if complete_status > 0 {
            assign(ENDPTCOMPLETE, 0xFFFF_FFFF);
        }
    }
    // debug_str(b"[usb] / irq serviced /");
}

fn noop() {}
