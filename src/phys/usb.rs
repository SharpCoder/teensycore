#![allow(dead_code)]

use crate::*;
use crate::debug::*;
use crate::mem::{zero};
use crate::{phys::*, assembly};
use crate::phys::addrs::*;
use crate::phys::irq::*;
use crate::phys::read_word;

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
pub const TI0: u32 = 1 << 24;
pub const TI1: u32 = 1 << 25;


pub struct UsbEndpointConfig {
    pub stall: bool,
    pub enabled: bool,
    pub reset: bool,
    pub endpoint_type: UsbEndpointType,
}

#[repr(C)]
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
    first_transfer: u32,
    last_transfer: u32,
    transfer_complete: Fn,
    unused: u32,
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

#[repr(C)]
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

const BLANK_QUEUE_HEAD: UsbEndpointQueueHead = UsbEndpointQueueHead { config: 0, current: 0, next: 0, status: 0, pointer0: 0, pointer1: 0, pointer2: 0, pointer3: 0, pointer4: 0, setup0: 0, setup1: 0, reserved: 0, first_transfer: 0, last_transfer: 0, transfer_complete: noop, unused: 0 };

#[link_section = ".endpoint_queue"]
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; 16] = [BLANK_QUEUE_HEAD; 16];
static mut INITIALIZED: bool = false;

fn usb_endpoint_location() -> u32 {
    unsafe {
        let endpoint0 = &ENDPOINT_HEADERS[0] as *const UsbEndpointQueueHead;
        return endpoint0 as u32;
    }
}

fn get_queue_head(queue: u32) -> *mut UsbEndpointQueueHead {
    if queue > 16 {
        // Invalid!!!
        crate::err(crate::PanicType::Oob);
    }

    let addr = usb_endpoint_location() + queue * 64;
    return addr as *mut UsbEndpointQueueHead;
}

pub fn usb_start_clock() {
    // Ungate the USB clock
    assign(0x400F_C000 + 0x80, read_word(0x400F_C000 + 0x80) | 0x3);
    // Writing 1 to this bit (31) will soft-reset the USBPHYS device
    assign(0x400D_9034, 0x80000000);
    // Reset the controller
    assign(USB + 0x140, 2);
    while read_word(USB + 0x140) & 0x2 > 0 {
        assembly!("nop");
    }
    // Writing 1 to this bit (31) will soft-reset the USBPHYS device
    // but different from the earlier one???
    assign(0x400D_9038, 1 << 31);
    // Wait 25ms
    wait_ns(25 * MS_TO_NANO);
    // Gate the USBPHYS n
    assign(0x400D_9038, 1 << 30);
    assign(0x400D_9000, 0);
}

pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            // Enter device mode and set the SLOM bit
            wait_ns(100);
            assign(USB + 0x1A8, 0x2 | (0x1 << 3));
        },
    }
}
/// Enable all usb interrupts
pub fn usb_irq_enable(value: u32) {
    irq_attach(Irq::Usb, handle_usb_irq);
    irq_enable(Irq::Usb);

    assign(USB + 0x144, 0xFFFFFFFF);
    assign(USB + 0x148, value);
}

pub fn usb_irq_clear(value: u32) {
    wait_exact_ns(1);
    assign(USB + 0x144, value);
}

/// Disable all usb interrupts
pub fn usb_irq_disable() {
    irq_disable(Irq::Usb);
    assign(USB + 0x148, 0x0);
}

pub fn usb_initialize_endpoints() {
    unsafe {
        let epaddr = usb_endpoint_location();
        // Zero out the memory block
        zero(epaddr, 2048);

        // Load pointers to the relevant things
        let endpoint0 = get_queue_head(0);
        let endpoint1 = get_queue_head(1);

        // Priming the headers
        // First, set max_packet_size
        let max_packet_size = 64;
        (*endpoint0).config |= max_packet_size << 16;
        (*endpoint1).config |= max_packet_size << 16;

        // Set interrupt-on-setup
        (*endpoint0).config |= 1 << 15;

        // // Write a 1 to the nextdtd pointer
        // ENDPOINT_HEADERS[0].next |= 1;
        // ENDPOINT_HEADERS[1].next |= 1;
        assign(USB + 0x158, epaddr);
        // Set burst size
        assign(USB + 0x160, 0x404);
        // Set the OTG Termination bit
        assign(USB + 0x1A4, read_word(USB + 0x1A4) | (0x1 << 3));

        debug_hex(epaddr, b"epaddr");

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
        value |=  0x1 << 6;
    }

    if config.enabled {
        value |= 0x1 << 7;
    }

    // Duplicate this configuration across TX and RX
    value |= value << 16;

    // assign(USB + 0x1C0 + (0x4 * endpoint), value);
    assign(USB + 0x1C0 + (0x4 * endpoint), value);

}

pub fn usb_restart() {
    // Send stop command
    assign(USB + 0x140, 0);
    // Send reset command
    assign(USB + 0x140, 0x2);
    // Send start command
    assign(USB + 0x140, 0x1);
}

pub fn usb_stop() {
    let original_value = read_word(USB + 0x140);
    assign(USB + 0x140, original_value & !0x1);
}

fn handle_usb_irq() {
    irq_disable(Irq::Usb);

    let irq_status = read_word(USB + 0x144);
    usb_irq_clear(irq_status);
    debug_hex(irq_status, b"[usb] irq received");
    
    // debug_binary(irq_status, b"[usb] irq status");
    // Check the setup status
    let setup_status = read_word(USB + 0x1AC);
    if setup_status > 0 {
        debug_str(b"[usb] setup_status detected");
    }

    let nak_status = read_word(USB + 0x178);
    if nak_status > 0 {
        debug_str(b"[usb] nak requested");
    }
    
    if (irq_status & HCH) > 0 {
        debug_str(b"[usb] DCHalted!!!!!!!!!");
    }

    let endpoint_primed = read_word(USB + 0x1B0);
    debug_binary(endpoint_primed, b"[usb] endpoint primed");
    debug_hex(read_word(USB + 0x1C0), b"[usb] endpoint0 control");

    if (irq_status & PCI) > 0 {
        debug_str(b" -> [usb] PCI flag detected");
        
        // Check which mode we are in
        let port_status = read_word(USB + 0x184);
        if port_status & (0x1 << 9) > 0 { 
            debug_str(b"[usb] highspeed mode");
        } else {
            debug_str(b"[usb] lowspeed");
        }

        debug_hex(read_word(USB + 0x184), b"[usb] port status");
        // Mark it as primed
        // let original = read_word(USB + 0x1B0);
        // assign(USB + 0x1B0, original | 0x1 | (0x1 << 16));
    }

    if (irq_status & SRI) > 0 {
        debug_str(b" -> [usb] SRI (start of frame) flag detected");
    }

    if (irq_status & USBERRINT) > 0 {
        // Error
        debug_str(b" -> [usb] USBERRINT flag detected");
    }
    
    if (irq_status & USBINT) > 0 {
        // 
        debug_str(b" -> [usb] USBINT flag detected");
    }

    if (irq_status & SLI) > 0 {
        debug_str(b" -> [usb] SLI flag detected");
    }
    
    if (irq_status & TI0) > 0 {
        debug_str(b" -> [usb] TI0 flag detected");
    }

    if (irq_status & TI1) > 0 {
        debug_str(b" -> [usb] TI1 flag detected");
    }

    if (irq_status & URI) > 0{

        if unsafe { INITIALIZED } == false {
            debug_str(b" -> [usb] URI flag detected");
            // Clear ENDPTSETUPSTAT
            assign(USB + 0x1AC, read_word(USB + 0x1AC));
            // Clear ENDPTCOMPLETE
            assign(USB + 0x1BC, read_word(USB + 0x1BC));

            // Wait for endpoint priming to finish
            while read_word(USB + 0x1B0) != 0 {
                assembly!("nop");
            }

            // Cancel all endpoint primed status flags
            assign(USB + 0x1B4, 0xFFFF_FFFF);

            // Read the reset bit and make sure it is still active
            let port_status = read_word(USB + 0x184);
            if (port_status & (0x1 << 8)) == 0 {
                debug_str(b"[usb] ERROR PORT STATUS");
            }

            // Do any other work
            // ...

            unsafe {
                INITIALIZED = true;
                assign(USB + 0x1B0, 0x1 | (0x1 << 16));
            }
        }
    } 

    // Output the packet of data for endpoint0RX
    unsafe {
        let endpoint0 = get_queue_head(0);
        debug_binary(read_word(USB + 0x1B8), b"Endpoint Status");
        debug_hex((*endpoint0).config, b"config");
        debug_hex((*endpoint0).current, b"current");
        debug_hex((*endpoint0).next, b"next");
        debug_hex((*endpoint0).status, b"status");
        debug_hex((*endpoint0).pointer0, b"pointer0");
        debug_hex((*endpoint0).pointer1, b"pointer1");
        debug_hex((*endpoint0).pointer2, b"pointer2");
        debug_hex((*endpoint0).pointer3, b"pointer3");
        debug_hex((*endpoint0).pointer4, b"pointer4");
        debug_hex((*endpoint0).setup0, b"setup0");
        debug_hex((*endpoint0).setup1, b"setup1");
        debug_hex((*endpoint0).first_transfer, b"first_transfer");
    }

    debug_str(b"[usb] irq serviced");
    irq_enable(Irq::Usb);
}

fn noop() { }