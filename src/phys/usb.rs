use crate::debug::{blink, blink_hardware};
use crate::{phys::*};
use crate::phys::addrs::*;

use crate::phys::irq::*;
use crate::phys::read_word;

pub enum UsbMode {
    DEVICE,
    // Not supported at this time
    // HOST,
}

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

const BlankUsbEndpointQueueHead: UsbEndpointQueueHead = UsbEndpointQueueHead { config: 0, current: 0, next: 0, status: 0, pointer0: 0, pointer1: 0, pointer2: 0, pointer3: 0, pointer4: 0, setup0: 0, setup1: 0, reserved: 0 };
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; 8] = [BlankUsbEndpointQueueHead; 8];

pub fn usb_start_clock() {
    assign(0x400F_C000 + 0x80, read_word(0x400F_C000 + 0x80) | 0x3);
}

/// Enable all usb interrupts
pub fn usb_irq_enable() {
    irq_attach(Irq::Usb, handle_usb_irq);
    irq_attach(Irq::UsbPhy1, handle_usb_irq);
    irq_attach(Irq::UsbPhy2, handle_usb_irq);
    irq_enable(Irq::Usb);
    irq_enable(Irq::UsbPhy1);
    irq_enable(Irq::UsbPhy2);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb);
    irq_disable(Irq::UsbPhy1);
    irq_disable(Irq::UsbPhy2);
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            assign(USB + 0x1A8, 0x2);
        },
    }
}

pub fn usb_initialize_endpoints() {
    unsafe {

        for i in 0 .. ENDPOINT_HEADERS.len() {
            ENDPOINT_HEADERS[i].clear();
        }

        let epaddr = &ENDPOINT_HEADERS as *const UsbEndpointQueueHead;
        assign(USB + 0x158, epaddr as u32);
    }
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