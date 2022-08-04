#![allow(dead_code)]

use crate::debug::{blink_hardware};
use crate::{phys::*};
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
    assign(0x400D_9038, (0x1 << 30));
}

/// Enable all usb interrupts
pub fn usb_irq_enable() {
    irq_attach(Irq::Usb, handle_usb_irq);
    irq_attach(Irq::UsbPhy1, handle_usb_irq);
    irq_attach(Irq::UsbPhy2, handle_usb_irq);
    irq_enable(Irq::Usb);
    irq_enable(Irq::UsbPhy1);
    irq_enable(Irq::UsbPhy2);

    // TODO: Allow configuring the interrupts individually
    assign(USB + 0x148, 0x30000FF);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb);
    irq_disable(Irq::UsbPhy1);
    irq_disable(Irq::UsbPhy2);

    assign(USB + 0x148, 0x0);
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            let original = read_word(USB + 0x1A8);
            assign(USB + 0x1A8, original | 0x2);
        },
    }
}

pub fn usb_initialize_endpoints() {
    unsafe {

        // TODO: Find out what this does...
        ENDPOINT_HEADERS[0].config = (64 << 16) | (1 << 15);
        ENDPOINT_HEADERS[1].config = (64 << 16);
        
        let epaddr = &mut ENDPOINT_HEADERS[0] as *mut UsbEndpointQueueHead;
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
        value |=  (0x1 << 6);
    }

    if config.enabled {
        value |= (0x1 << 7);
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
    assign(USB + 0x140, original_value | 0b1);
}

pub fn usb_stop() {
    let original_value = read_word(USB + 0x140);
    assign(USB + 0x140, original_value & !0b1);
}

fn handle_usb_irq() {
    blink_hardware(5);
}