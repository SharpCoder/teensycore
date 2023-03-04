#![allow(dead_code)]

pub mod descriptors;
pub mod models;
pub mod registers;

use descriptors::*;
use models::*;
use registers::*;

use crate::mem::zero;
use crate::phys::addrs::*;
use crate::phys::irq::*;
use crate::phys::read_word;
use crate::system::vector::*;
use crate::*;
use crate::{assembly, phys::*};

type IrqFn = fn(status: u32);

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
    first_transfer: 0,
    last_transfer: 0,
};

const MAX_ENDPOINTS: usize = 8;

#[no_mangle]
#[link_section = ".endpoint_queue"]
static mut ENDPOINT_HEADERS: [UsbEndpointQueueHead; MAX_ENDPOINTS * 2] =
    [BLANK_QUEUE_HEAD; MAX_ENDPOINTS * 2];

#[link_section = ".dmabuffers"]
static mut USB_DESCRIPTOR_BUFFER: BufferPage = BufferPage::new();

#[no_mangle]
#[link_section = ".dmabuffers"]
static mut ENDPOINT0_BUFFER: BufferPage = BufferPage::new();

#[no_mangle]
static mut ENDPOINT0_TRANSFER_DATA: UsbEndpointTransferDescriptor =
    UsbEndpointTransferDescriptor::new();

#[no_mangle]
static mut ENDPOINT0_TRANSFER_ACK: UsbEndpointTransferDescriptor =
    UsbEndpointTransferDescriptor::new();

static mut ENDPOINT0_NOTIFY_MASK: u32 = 0;
static mut IRQ_CALLBACKS: Vector<IrqFn> = Vector::new();
static mut CONFIGURATION_CALLBACKS: Vector<ConfigFn> = Vector::new();
static mut CONFIGURATION: u16 = 0;
static mut HIGHSPEED: bool = false;

/// Attach a callback to be invoked when a setup packet
/// is received. See usb_serial.rs for examples.
pub fn usb_attach_setup_callback(callback: ConfigFn) {
    unsafe {
        CONFIGURATION_CALLBACKS.push(callback);
    }
}

/// Attach a callback to be invoked every time an
/// interrupt is handled.
pub fn usb_attach_irq_handler(callback: IrqFn) {
    unsafe {
        IRQ_CALLBACKS.push(callback);
    }
}

/// Configure the VendorID and ProductID
/// of the peripheral.
pub fn usb_configure_codes(vid: u16, pid: u16) {
    let descriptors = usb_get_descriptors();
    descriptors.set_codes(vid, pid);
}

/// Retrieve the specific queue head based on endpoint
/// and direction.
///
/// Returns a mutable UsbEndpointQueueHead.
pub fn usb_get_queuehead(endpoint: usize, tx: bool) -> &'static mut UsbEndpointQueueHead {
    unsafe {
        let index = match tx {
            true => endpoint * 2 + 1,
            false => endpoint * 2,
        };

        return &mut ENDPOINT_HEADERS[index];
    }
}

/// Boot up the usb clocks. This method
/// takes many milliseconds to run.
pub fn usb_start_clock() {
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

/// This method will initialize the usb subsystem by priming
/// the endpoint queues, starting irq, and enabling the
/// run/stop bit of the USB OTG1 Core.
pub fn usb_initialize() {
    // Configure the descriptors
    usb_initialize_descriptors();

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

    assign(USBINTR, 0x143);

    irq_attach(Irq::Usb1, handle_usb_irq);
    irq_priority(Irq::Usb1, 32);
    irq_enable(Irq::Usb1);

    usb_cmd(1); // Run/Stop bit
}

/// Set the mode of the USB device.
pub fn usb_set_mode(mode: UsbMode) {
    match mode {
        UsbMode::DEVICE => {
            // Enter device mode and set the SLOM bit
            assign(USBMODE, 0x2 | (0x1 << 3));
        }
    }
}

/// Wait for a register to read all 0's across the borad
fn waitfor(addr: u32) {
    while read_word(addr) > 0 {
        assembly!("nop");
    }
}

/// Clear specific interrupts from the status field.
pub fn usb_irq_clear(value: u32) {
    assign(USBSTS, value);
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
fn configure_ep(qh: &mut UsbEndpointQueueHead, config: u32, cb: Option<TransferCallbackFn>) {
    qh.config = config;
    qh.next = 1;

    if cb.is_some() {
        qh.callback = cb.unwrap();
    } else {
        qh.callback = noop;
    }
}

fn run_callbacks(qh: &mut UsbEndpointQueueHead) {
    let mut transfer_addr = qh.first_transfer;
    while transfer_addr > 1 {
        // Get the transfer
        let transfer = unsafe {
            (transfer_addr as *const UsbEndpointTransferDescriptor)
                .as_ref()
                .unwrap()
        };

        // Still active
        if (transfer.status & 0x80) > 0 {
            qh.first_transfer = transfer_addr;
            break;
        }

        // Check for the end of queue.
        if transfer.next == 1 {
            // Reset queueheads
            qh.first_transfer = 0;
            qh.last_transfer = 0;
        } else {
            transfer_addr = transfer.next;
        }

        // Invoke the callback
        qh.callback.call((transfer,));

        // If we did reach the end of the queue, stop processing.
        if transfer.next == 1 {
            break;
        }
    }
}

/// Use this method to fully configure an endpoint
pub fn usb_setup_endpoint(
    index: usize,
    tx_config: Option<EndpointConfig>,
    rx_config: Option<EndpointConfig>,
) {
    let tx_qh = usb_get_queuehead(index, true);
    let rx_qh = usb_get_queuehead(index, false);
    let ep_control_addr = USB + 0x1C0 + (index as u32) * 4;

    let isochornous = 1;
    let bulk = 2;
    let interrupt = 3;
    let tx_enable_bit = 1 << 23;
    let rx_enable_bit = 1 << 7;

    if tx_config.is_some() {
        let config = tx_config.unwrap();

        let mut config_bits = (config.size as u32) << 16;
        if config.zlt {
            config_bits |= 1 << 29;
        }

        configure_ep(tx_qh, config_bits, config.callback);
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
    } else {
        // Unusued endpoints cannot remain CONTROL type
        assign(ep_control_addr, read_word(ep_control_addr) | (bulk << 18));
    }

    if rx_config.is_some() {
        let config = rx_config.unwrap();

        let mut config_bits = (config.size as u32) << 16;
        if config.zlt {
            config_bits |= 1 << 29;
        }

        configure_ep(rx_qh, config_bits, config.callback);
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
    } else {
        // Unusued endpoints cannot remain CONTROL type
        assign(ep_control_addr, read_word(ep_control_addr) | (bulk << 2));
    }
}

/// This method will configure a UsbEndpointTransferDescriptor
/// based on various flags. However, DTD structures can
/// only be modified while they are not in an active state.
///
/// This method returns true if it succeeded in modifying the dtd
/// or false if something went wrong.
pub fn usb_prepare_transfer(
    transfer_queue: &mut UsbEndpointTransferDescriptor,
    addr: u32,
    len: u32,
    notify: bool,
) -> bool {
    if (transfer_queue.status & 0x80) == 0 {
        transfer_queue.next = 1;
        transfer_queue.status = (len << 16) | (1 << 7);
        transfer_queue.pointer0 = addr;
        transfer_queue.pointer1 = addr + 4096;
        transfer_queue.pointer2 = addr + 8192;
        transfer_queue.pointer3 = addr + 12288;
        transfer_queue.pointer4 = addr + 16384;

        if notify {
            transfer_queue.status |= 1 << 15;
        }

        return true;
    }

    // Something went wrong.
    return false;
}

/// This method will enqueue the transfer descriptor into the endpoint
/// and prime it for receiving new data.
pub fn usb_receive(endpoint: usize, transfer: &mut UsbEndpointTransferDescriptor) {
    schedule_transfer(endpoint as u32, false, transfer);
}

/// This method will enqueue the transfer descriptor into the endpoint
/// and prime it for transmitting new data.
pub fn usb_transmit(endpoint: usize, transfer: &mut UsbEndpointTransferDescriptor) {
    schedule_transfer(endpoint as u32, true, transfer);
}

fn schedule_transfer(ep: u32, tx: bool, transfer: &mut UsbEndpointTransferDescriptor) {
    let qh = usb_get_queuehead(ep as usize, tx);
    let mask = match tx {
        true => 1 << (ep + 16),
        false => 1 << ep,
    };

    loop {
        // Case 2. The queue is not empty.
        if qh.last_transfer > 1 {
            let last = qh.get_last_transfer();

            // Add the new dtd to the end of the queue
            last.next = (transfer as *const UsbEndpointTransferDescriptor) as u32;

            // If the thing is still primed, hooray we're done.
            if (read_word(ENDPTPRIME) & mask) > 0 {
                break;
            }

            let mut status;
            loop {
                // Set ATDTW bit to USBCMD
                assign(USBCMD, read_word(USBCMD) | (1 << 14));
                // Read status for current queue
                status = read_word(ENDPTSTAT) & mask;
                // Read atdtw bit
                let atdtw = read_word(USBCMD) & (1 << 14);
                // If it's zero, restart this process.
                // If it's one, we can continue.
                if atdtw > 0 {
                    break;
                } else {
                    assembly!("nop");
                }
            }

            // Write atdtw as zero
            assign(USBCMD, read_word(USBCMD) & !(1 << 14));

            // If status bit is set, we're done. Otherwise, fall into Case 1.
            if status > 0 {
                break;
            }
        }

        // Case 1. The queue is empty
        qh.next = (transfer as *const UsbEndpointTransferDescriptor) as u32;
        qh.status = 0;

        usb_prime_endpoint(ep, tx);
        qh.set_first_transfer(transfer);
        break;
    }

    qh.set_last_transfer(transfer);
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
            return;
        } else {
            assembly!("nop");
        }
    }
}

fn endpoint0_setup(packet: SetupPacket) {
    for callback in unsafe { CONFIGURATION_CALLBACKS.into_iter() } {
        callback(packet);
    }

    match packet.bm_request_and_type {
        0x681 | 0x680 => {
            // GET_DESCRIPTOR
            let descriptors = usb_get_descriptors();
            match descriptors.get_bytes(packet.w_value, packet.w_index) {
                Some(bytes) => {
                    let mut byte_length = bytes.size();
                    if byte_length > packet.w_length as usize {
                        byte_length = packet.w_length as usize;
                    }

                    endpoint0_transmit(bytes, byte_length, false);
                    return;
                }
                None => {}
            }
        }
        0x500 => {
            // Set Address
            endpoint0_receive(0, 0, false);
            assign(DEVICEADDR, ((packet.w_value as u32) << 25) | (1 << 24));
            return;
        }
        0x900 => {
            // Set configuration
            unsafe {
                CONFIGURATION = packet.w_value;
            }

            endpoint0_receive(0, 0, false);
            return;
        }
        0x880 => {
            // Get configuration
        }
        0x80 => {
            // Get status (device)
        }
        0x82 => {
            // Get status (endpoint)
        }
        0x302 => {
            // Set feature
        }
        0x102 => {
            // Clear feature
        }
        0x2021 => {
            // Set Line Coding
            if packet.w_length != 7 {
                // Stall
                assign(ENDPTCTRL0, (1 << 16) | 1); // Stall
                return;
            }

            endpoint0_receive(unsafe { ENDPOINT0_BUFFER.as_ptr() } as u32, 7, true);
            return;
        }
        0x2221 => {
            //Set control line state
            endpoint0_receive(0, 0, false);
            return;
        }
        0x2321 => {
            //Send Break
            endpoint0_receive(0, 0, false);
            return;
        }
        _ => {}
    }

    assign(ENDPTCTRL0, (1 << 16) | 1); // Stall
}

fn endpoint0_transmit(vec: Vector<u8>, byte_length: usize, notify: bool) {
    // Do the transmit
    let len = byte_length as u32;
    let usb_descriptor_buffer_addr = unsafe { USB_DESCRIPTOR_BUFFER.as_ptr() } as u32;
    arm_dcache_delete(usb_descriptor_buffer_addr, len);
    let mut i = 0;
    for byte in vec.into_iter() {
        unsafe {
            USB_DESCRIPTOR_BUFFER.bytes[i] = byte;
        }

        i += 1;
    }

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

    if notify {
        unsafe {
            ENDPOINT0_NOTIFY_MASK = 1 << 16;
        }
    }

    assign(ENDPTCOMPLETE, 1 | (1 << 16));
    usb_prime_endpoint(0, true);
}

fn handle_usb_irq() {
    irq_disable(Irq::Usb1);

    let irq_status = read_word(USBSTS);
    assembly!("nop"); // Need this. no idea why.
    usb_irq_clear(irq_status);

    if (irq_status & HCH) > 0 {}

    if (irq_status & PCI) > 0 {
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
        }
    }

    if (irq_status & SEI) > 0 {
        // System error flag
    }

    if (irq_status & USBERRINT) > 0 {
        // Interrupt error flag
    }

    if (irq_status & SLI) > 0 {
        // Enter suspend mode
    }

    if (irq_status & URI) > 0 {
        // Reset device
        assign(ENDPTSTAT, read_word(ENDPTSTAT));
        assign(ENDPTCOMPLETE, read_word(ENDPTCOMPLETE));

        // Wait for endpoint priming to finish
        waitfor(ENDPTPRIME);

        // Flush all endpoints
        assign(ENDPTFLUSH, 0xFFFF_FFFF);

        // Read the reset bit and make sure it is still active
        let port_status = read_word(PORTSC1);
        if (port_status & (1 << 8)) == 0 {
            // ERROR PORT STATUS
        } else {
            // Still active
        }
    }

    if (irq_status & USBINT) > 0 {
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
            waitfor(ENDPTFLUSH);

            if (read_word(ENDPTSTAT) & ((1 << 16) | 1)) > 0 {
                // FLUSH FAILED
                // TODO: Something
            }

            // Setup
            endpoint0_setup(setup_packet);

            // Check for another packet
            setup_status = read_word(ENDPTSETUPSTAT);
        }

        let complete_status = read_word(ENDPTCOMPLETE);

        unsafe {
            if (complete_status & ENDPOINT0_NOTIFY_MASK) > 0 {
                ENDPOINT0_NOTIFY_MASK = 0;
                endpoint0_complete();
            }
        }

        if complete_status > 0 {
            assign(ENDPTCOMPLETE, complete_status);

            // Run the transmit callbacks
            for idx in 1..MAX_ENDPOINTS {
                let mask = 1 << (16 + idx);
                if (complete_status & mask) > 0 {
                    run_callbacks(usb_get_queuehead(idx, true));
                }
            }

            // Run the receive callbacks
            for idx in 1..MAX_ENDPOINTS {
                let mask = 1 << idx;
                if (complete_status & mask) > 0 {
                    run_callbacks(usb_get_queuehead(idx, false));
                }
            }
        }
    }

    unsafe {
        for other_irq_handler in IRQ_CALLBACKS.into_iter() {
            other_irq_handler(irq_status);
        }
    }

    irq_enable(Irq::Usb1);
}

fn endpoint0_complete() {
    // TODO: This is not always what endpoint0_complete means
    // choose correct action based on request.

    // Read the buffer
    let buffer = unsafe { ENDPOINT0_BUFFER.bytes };
    let mut _bitrate = 0;

    for i in 0..4 {
        _bitrate |= (buffer[i] as u64) << (i * 8);
    }
}

fn noop(_packet: &UsbEndpointTransferDescriptor) {}
