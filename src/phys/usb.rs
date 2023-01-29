#![allow(dead_code)]

use crate::debug::*;
use crate::mem::zero;
use crate::phys::addrs::*;
use crate::phys::irq::*;
use crate::phys::read_word;
use crate::*;
use crate::{assembly, phys::*};

type Fn = fn();

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
pub const SLI: u32 = 1 << 8;
pub const HCH: u32 = 1 << 12;
pub const NAKE: u32 = 1 << 16;
pub const TI0: u32 = 1 << 24;
pub const TI1: u32 = 1 << 25;

/** Registers */

pub const USBCMD: u32 = 0x402E_0140;
pub const USBSTS: u32 = 0x402E_0144;
pub const USBINTR: u32 = 0x402E_0148;
pub const ENDPTLISTADDR: u32 = 0x402E_0158;
pub const USBMODE: u32 = 0x402E_01A8;
pub const PORTSC1: u32 = 0x402E_0184;
pub const ENDPTSETUPSTAT: u32 = 0x402E_01AC;
pub const ENDPTPRIME: u32 = 0x402E_01B0;
pub const ENDPTFLUSH: u32 = 0x402E_01B4;
pub const ENDPTSTAT: u32 = 0x402E_01B8;
pub const ENDPTCOMPLETE: u32 = 0x402E_01BC;
pub const ENDPTCTRL0: u32 = 0x402E_01C0;

/************************/

pub struct UsbEndpointConfig {
    pub stall: bool,
    pub enabled: bool,
    pub reset: bool,
    pub endpoint_type: UsbEndpointType,
}

#[repr(C, align(64))]
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
    // first_transfer: u32,
    // last_transfer: u32,
    // transfer_complete: Fn,
    // unused: u32,
}

// impl UsbEndpointQueueHead {
//     fn convert_to_dtd(addr: u32) -> *mut UsbEndpointTransferDescriptor {
//         return addr as *mut UsbEndpointTransferDescriptor;
//     }

//     pub fn get_dtd(self) -> *mut UsbEndpointTransferDescriptor {
//         return UsbEndpointQueueHead::convert_to_dtd(self.current);
//     }

//     pub fn get_next_dtd(self) -> *mut UsbEndpointTransferDescriptor {
//         return UsbEndpointQueueHead::convert_to_dtd(self.next);
//     }
// }

#[repr(C)]
struct UsbEndpointTransferDescriptor {
    next: u32,
    status: u32,
    pointer0: u32,
    pointer1: u32,
    pointer2: u32,
    pointer3: u32,
    pointer4: u32,
    callback: Fn,
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
    // first_transfer: 0,
    // last_transfer: 0,
    // transfer_complete: noop,
    // unused: 0,
};

#[no_mangle]
#[link_section = ".endpoint_queue"]
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; 16] = [BLANK_QUEUE_HEAD; 16];
static mut INITIALIZED: bool = false;
static mut HIGHSPEED: bool = false;

fn usb_endpoint_location(queue: usize) -> u32 {
    unsafe {
        let endpoint = &ENDPOINT_HEADERS[queue] as *const UsbEndpointQueueHead;
        return endpoint as u32;
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
    usb_initialize_endpoints();
    usb_irq_enable(0x143); // 0x143, 0x30105FF
    usb_cmd(1); // Run/Stop bit

    debug_str(b"[usb] booting...");
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            // Enter device mode and set the SLOM bit
            assign(USB + 0x1A8, 0x2 | (0x1 << 3));
        }
    }
}
/// Enable all usb interrupts
pub fn usb_irq_enable(value: u32) {
    assign(USBINTR, value);

    irq_attach(Irq::Usb1, handle_usb_irq);
    irq_enable(Irq::Usb1);
}

pub fn usb_irq_clear(value: u32) {
    wait_exact_ns(1);
    assign(USBSTS, value);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb1);
    assign(USBINTR, 0x0);
}

pub fn usb_initialize_endpoints() {
    unsafe {
        let epaddr = usb_endpoint_location(0);
        zero(epaddr, 4096 / 4);

        // Priming the headers
        // First, set max_packet_size
        ENDPOINT_HEADERS[0].config |= (64 << 16) | (1 << 15); // RX
        ENDPOINT_HEADERS[1].config |= 64 << 16; // TX

        assign(ENDPTLISTADDR, epaddr);

        // Set the OTG Termination bit
        // assign(USB + 0x1A4, read_word(USB + 0x1A4) | (0x1 << 3));
    }
}

pub fn usb_configure_endpoint(endpoint: u32, config: UsbEndpointConfig) {
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
        value |= 0x1 << 6;
    }

    if config.enabled {
        value |= 0x1 << 7;
    }

    // Duplicate this configuration across TX and RX
    value |= value << 16;

    // assign(USB + 0x1C0 + (0x4 * endpoint), value);
    assign(USB + 0x1C0 + (0x4 * endpoint), value);
}

pub fn usb_cmd(val: u32) {
    assign(USBCMD, val);
}

#[no_mangle]
fn handle_usb_irq() {
    let show_messages = true;
    let irq_status = read_word(USBSTS);
    usb_irq_clear(irq_status);

    // debug_hex(usb_endpoint_location(), b"usb endpoint location");

    // Check the setup status
    let nak_status = read_word(USB + 0x178);
    if nak_status > 0 {
        debug_str(b"[usb] nak requested");
    }

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
            debug_str(b"   highspeed");
        } else {
            unsafe {
                HIGHSPEED = false;
            }
        }

        if (port_status & 1) > 0 {
            // Attached
            debug_str(b"attached");
        }

        // debug_hex(port_status, b"port status");
        // if port_status & (0x1 << 7) > 0 {
        //     assign(PORTSC1, read_word(PORTSC1) & (0x1 << 6));
        // }
    }

    if (irq_status & FRI) > 0 {
        if show_messages {
            debug_str(b" -> [usb] frame rollover flag detected");
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
        //     assign(USBSTS, SRI);
    }

    if (irq_status & USBERRINT) > 0 {
        debug_str(b" -> [usb] USBERRINT flag detected");
    }

    if (irq_status & USBINT) > 0 {
        debug_str(b" -> [usb] USBINT flag detected");

        loop {
            let setup_status = read_word(USB + 0x1AC);
            if setup_status > 0 {
            } else {
                break;
            }
        }

        if setup_status > 0 {
            debug_str(b"[usb] setup_status detected");

            unsafe {
                debug_hex(ENDPOINT_HEADERS[0].config, b"config");
                debug_hex(ENDPOINT_HEADERS[0].current, b"current");
                debug_hex(ENDPOINT_HEADERS[0].next, b"next");
                debug_hex(ENDPOINT_HEADERS[0].status, b"status");
                debug_hex(ENDPOINT_HEADERS[0].pointer0, b"pointer0");
                debug_hex(ENDPOINT_HEADERS[0].pointer1, b"pointer1");
                debug_hex(ENDPOINT_HEADERS[0].pointer2, b"pointer2");
                debug_hex(ENDPOINT_HEADERS[0].pointer3, b"pointer3");
                debug_hex(ENDPOINT_HEADERS[0].pointer4, b"pointer4");
                debug_hex(ENDPOINT_HEADERS[0].setup0, b"setup0");
                debug_hex(ENDPOINT_HEADERS[0].setup1, b"setup1");
            }
        }
    }

    if (irq_status & SLI) > 0 && (read_word(PORTSC1) & (0x1 << 7) > 0) {
        if show_messages {
            debug_str(b" -> [usb] Enter sleep mode");
        }

        // assign(PORTSC1, 0x1 << 6);
    }

    if (irq_status & TI0) > 0 {
        debug_str(b" -> [usb] TI0 flag detected");
    }

    if (irq_status & TI1) > 0 {
        debug_str(b" -> [usb] TI1 flag detected");
    }

    if (irq_status & URI) > 0 {
        if unsafe { INITIALIZED } == false {
            debug_str(b" -> [usb] URI flag detected");
            assign(ENDPTSTAT, read_word(USB + 0x1B8));
            assign(ENDPTCOMPLETE, read_word(USB + 0x1BC));

            // Wait for endpoint priming to finish
            while read_word(ENDPTPRIME) != 0 {
                assembly!("nop");
            }

            // Flush all endpoints
            assign(ENDPTFLUSH, 0xFFFFFFFF);

            // Read the reset bit and make sure it is still active
            let port_status = read_word(PORTSC1);
            if (port_status & (1 << 8)) == 0 {
                debug_str(b"[usb] ERROR PORT STATUS");
            } else {
                // Still active
            }

            // Do any other work
            // ...

            // unsafe {
            // INITIALIZED = true;
            // }

            assign(ENDPTPRIME, 0x2 | (0x2 << 16));
        }
    }
}

fn noop() {}
