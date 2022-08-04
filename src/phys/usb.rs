#![allow(dead_code)]

use crate::*;
use crate::debug::{blink_hardware};
use crate::{phys::*, assembly};
use crate::phys::addrs::*;
use crate::phys::irq::*;
use crate::phys::read_word;

pub enum UsbEndpointDirection {
    TX,
    RX,
}

pub enum UsbEndpointType {
    CONTROL,
    ISOCHRONOUS,
    BULK,
    INTERRUPT,
}

pub enum UsbMode {
    DEVICE,
    // Not supported at this time
    // HOST,
}

pub const USBINT: u32 = 1;
pub const USBERRINT: u32 = 2;
pub const PCI: u32 = 1 << 2;
pub const FRI: u32 = 1 << 3;
pub const SEI: u32 = 1 << 4;
pub const URI: u32 = 1 << 6;
pub const SRI: u32 = 1 << 7;
pub const TI0: u32 = 1 << 24;
pub const TI1: u32 = 1 << 25;

// pub struct UsbIrqConfig {
//     pub usb: bool, // transfer description completed
//     pub usb_error: bool, // error condition
//     pub port_change: bool, // port change detect
//     pub frame_list_rollover: bool, // frame list rollover
//     pub system_err: bool, // system error
//     pub reset_received: bool, //usb reset received
// }

pub struct UsbEndpointConfig {
    pub stall: bool,
    pub enabled: bool,
    pub reset: bool,
    pub endpoint_type: UsbEndpointType,
}

#[repr(align(64))]
struct UsbEndpointQueueHead {
    config: u32,
    current: u32,
    next: u32,
    status: u32,
    pointer0: u32,
    pointer1: u32,
    pointer2: u32,
    pointer3: u32,
    pointer4: u32,
    reserved: u32,
    setup0: u32,
    setup1: u32,
}

impl UsbEndpointQueueHead {
    
    fn convert_to_dtd(addr: u32) -> *mut UsbEndpointTransferDescriptor {
        return addr as *mut UsbEndpointTransferDescriptor;
    }

    pub fn get_dtd(self) -> *mut UsbEndpointTransferDescriptor {
        return UsbEndpointQueueHead::convert_to_dtd(self.current);
    }

    pub fn get_next_dtd(self) -> *mut UsbEndpointTransferDescriptor {
        return UsbEndpointQueueHead::convert_to_dtd(self.next);
    }


}

struct UsbEndpointTransferDescriptor {
    next: u32,
    status: u32,
    pointer0: u32,
    pointer1: u32,
    pointer2: u32,
    pointer3: u32,
    pointer4: u32,
}

impl UsbEndpointQueueHead {
    pub fn clear(&mut self) {
        self.config = 0;
        self.current = 0;
        self.next = 0;
        self.status = 0;
        self.pointer0 = 0;
        self.pointer1 = 0;
        self.pointer2 = 0;
        self.pointer3 = 0;
        self.pointer4 = 0;
        self.setup0 = 0;
        self.setup1 = 0;
    }
}

const BLANK_QUEUE_HEAD: UsbEndpointQueueHead = UsbEndpointQueueHead { config: 0, current: 0, next: 0, status: 0, pointer0: 0, pointer1: 0, pointer2: 0, pointer3: 0, pointer4: 0, setup0: 0, setup1: 0, reserved: 0 };
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; 8] = [BLANK_QUEUE_HEAD; 8];

pub fn usb_start_clock() {
    // Ungate the USB clock
    assign(0x400F_C000 + 0x80, read_word(0x400F_C000 + 0x80) | 0x3);

    // GATE the USBPHYS clock
    // IDK why??? Everything breaks otherwise though.
    assign(0x400D_9038, 1 << 30);
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            let original = read_word(USB + 0x1A8);
            assign(USB + 0x1A8, original | 0x2 | 1 << 3);
        },
    }
}
/// Enable all usb interrupts
pub fn usb_irq_enable(value: u32) {
    irq_attach(Irq::Usb, handle_usb_irq);
    irq_attach(Irq::UsbPhy1, handle_usb_irq);
    irq_attach(Irq::UsbPhy2, handle_usb_irq);
    irq_enable(Irq::Usb);
    irq_enable(Irq::UsbPhy1);
    irq_enable(Irq::UsbPhy2);

    assign(USB + 0x148, value);
}

pub fn usb_irq_clear(value: u32) {
    assign(USB + 0x144, value);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb);
    irq_disable(Irq::UsbPhy1);
    irq_disable(Irq::UsbPhy2);

    assign(USB + 0x148, 0x0);
}

pub fn usb_initialize_endpoints() {
    unsafe {

        // Priming the headers
        // First, set max_packet_size
        let max_packet_size = 64;
        ENDPOINT_HEADERS[0].config |= max_packet_size << 16;
        ENDPOINT_HEADERS[1].config |= max_packet_size << 16;

        // Set interrupt-on-setup
        ENDPOINT_HEADERS[0].config |= 1 << 15;
        ENDPOINT_HEADERS[1].config |= 1 << 15;

        // Write a 1 to the nextdtd pointer
        ENDPOINT_HEADERS[0].next |= 1;
        ENDPOINT_HEADERS[1].next |= 1;

        let epaddr = &mut ENDPOINT_HEADERS as *mut UsbEndpointQueueHead;
        assign(USB + 0x158, epaddr as u32);
    }
}

pub fn usb_configure_endpoint(endpoint: u32, direction: UsbEndpointDirection, config: UsbEndpointConfig) {
    let mut value = 0;
    if config.stall {
        value |= 0x1;
    }

    if endpoint != 0 {
        value |= match config.endpoint_type {
            UsbEndpointType::CONTROL => 0x0,
            UsbEndpointType::ISOCHRONOUS => 0x1 << 3,
            UsbEndpointType::BULK => 0x2 << 3,
            UsbEndpointType::INTERRUPT => 0x3 << 3,
        };
    } // Otherwise, it has to be control - per the manual

    if config.reset {
        value |=  0x1 << 6;
    }

    if config.enabled {
        value |= 0x1 << 7;
    }

    match direction {
        UsbEndpointDirection::TX => {
            value <<= 16;
        },
        _ => {}
    }

    assign(USB + 0x1C0 + (0x4 * endpoint), value);

}

pub fn usb_start() {
    let original_value = read_word(USB + 0x140);
    assign(USB + 0x140, original_value | 0x1);
}

pub fn usb_stop() {
    let original_value = read_word(USB + 0x140);
    assign(USB + 0x140, original_value & !0x1);
}

fn handle_usb_irq() {
    let mut irq_clear_flags = 0;
    let irq_status = read_word(USB + 0x144);
    
    // Check the setup status
    let setup_status = read_word(USB + 0x1AC);
    if setup_status > 0 {
        blink_hardware(3);
    }

    if (irq_status & URI) > 0{
        // Clear ENDPTSETUPSTAT
        assign(USB + 0x1AC, 0xFFFF);
        // Clear ENDPTCOMPLETE
        assign(USB + 0x1BC, (0xFF << 16) |  0xFF);

        // Wait for endpoint priming to finish
        while read_word(USB + 0x1B0) > 0 {
            assembly!("nop");
        }

        // Cancel all endpoint primed status flags
        assign(USB + 0x1B4, (0xFF << 16) | 0xFF);

        // Do any other work
        // ...
        irq_clear_flags |= URI;
    } 
    
    if (irq_status & PCI) > 0 {
        irq_clear_flags |= PCI;
        
    }

    if (irq_status & USBERRINT) > 0 {
        // Error
        irq_clear_flags |= USBERRINT;
    }
    
    if (irq_status & USBINT) > 0 {
        // 
        blink_hardware(3);
        irq_clear_flags |= USBINT;
    }
    
    usb_irq_clear(irq_clear_flags);
}