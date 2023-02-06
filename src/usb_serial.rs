use crate::{
    arm_dcache_delete,
    debug::{debug_hex, debug_str},
    mem,
    phys::{
        addrs::USB,
        irq::{irq_disable, irq_enable},
        usb::models::*,
        usb::registers::*,
    },
    phys::{assign, irq::Irq, pins::pin_out, read_word, usb::descriptors::*, usb::*},
    serio::{serial_write, SerioDevice},
    system::{
        buffer::*,
        vector::{Array, Queue, Stack, Vector},
    },
    wait_exact_ns,
};

// How many pages of data we support
const RX_BUFFER_SIZE: usize = 512;
const RX_COUNT: usize = 4;
const TX_COUNT: usize = 4;

static mut BUFFER: Buffer<512, u8> = Buffer::new(0);

#[used]
#[link_section = ".descriptors"]
static mut TX_DTD: [UsbEndpointTransferDescriptor; TX_COUNT] =
    [UsbEndpointTransferDescriptor::new(); TX_COUNT];

#[used]
#[link_section = ".descriptors"]
static mut RX_DTD: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor::new();

const TX_BUFFER_SIZE: usize = 512;

#[used]
static mut TX_BUFFER_TRANSIENT: Buffer<512, u8> = Buffer::new(0);

#[used]
#[link_section = ".dmabuffers"]
static mut RX_BUFFER: BufferPage = BufferPage::new();

#[used]
#[link_section = ".dmabuffers"]
static mut TX_BUFFER: [BufferPage; TX_COUNT] = [BufferPage::new(); TX_COUNT];

#[used]
static mut TX_AVAILABLE: usize = 0;

const CDC_STATUS_INTERFACE: u8 = 0;
const CDC_DATA_INTERFACE: u8 = 1;
const CDC_ACM_SIZE: u16 = 16;
const CDC_RX_SIZE_480: u16 = 512;
const CDC_TX_SIZE_480: u16 = 512;
const CDC_RX_SIZE_12: u16 = 64;
const CDC_TX_SIZE_12: u16 = 64;
const CDC_ACM_ENDPOINT: u8 = 2;
const CDC_RX_ENDPOINT: u8 = 3;
const CDC_TX_ENDPOINT: u8 = 4;

pub fn usb_serial_init() {
    setup_cdc_descriptors();

    // Hook in the various callbacks
    usb_attach_setup_callback(usb_serial_configure);

    // Attach irq handler
    usb_attach_irq_handler(handle_irq);

    // Configure timer
    assign(USB + 0x80, 0x0003E7); // 1ms
    assign(USBINTR, read_word(USBINTR) | TI0);
}

fn usb_timer_oneshot() {
    assign(USB + 0x84, (1 << 31) | (1 << 30) | 0x0003E7);
}

fn handle_irq(irq_status: u32) {
    if (irq_status & TI0) > 0 {
        usb_serial_flush();
    }
}

fn usb_serial_configure(packet: SetupPacket) {
    match packet.bm_request_and_type {
        // SET_CONFIGURATION packet
        0x900 => {
            // Configure the endpoints.
            debug_str(b"configure endpoints from usb_serial");

            usb_setup_endpoint(
                CDC_ACM_ENDPOINT as usize,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::INTERRUPT,
                    size: CDC_ACM_SIZE,
                    zlt: false,
                    callback: None,
                }),
                None,
            );

            // Setup the serial transmit endpoint
            usb_setup_endpoint(
                CDC_TX_ENDPOINT as usize,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::BULK,
                    size: CDC_TX_SIZE_480,
                    zlt: false,
                    callback: Some(tx_callback),
                }),
                None,
            );

            // Setup the serial receive endpoint
            usb_setup_endpoint(
                CDC_RX_ENDPOINT as usize,
                None,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::BULK,
                    size: CDC_RX_SIZE_480,
                    zlt: false,
                    callback: Some(rx_callback),
                }),
            );

            // Clear
            mem::zero(
                unsafe { &TX_DTD as *const UsbEndpointTransferDescriptor } as u32,
                2048,
            );

            mem::zero(
                unsafe { &RX_DTD as *const UsbEndpointTransferDescriptor } as u32,
                2048,
            );

            rx_queue_transfer();
        }
        _ => {
            // Do nothing
            return;
        }
    }
}

fn rx_queue_transfer() {
    irq_disable(Irq::Usb1);
    let page = 0;
    let rx_buffer_len = RX_BUFFER_SIZE as u32;
    let base_address = unsafe { RX_BUFFER.as_ptr() } as u32;
    let page_address = base_address + (RX_BUFFER_SIZE * page) as u32;

    arm_dcache_delete(page_address, rx_buffer_len);
    usb_prepare_transfer(unsafe { &mut RX_DTD }, page_address, rx_buffer_len, true);
    usb_receive(CDC_RX_ENDPOINT as usize, unsafe { &mut RX_DTD });
    irq_enable(Irq::Usb1);
}

fn rx_callback(packet: &UsbEndpointTransferDescriptor) {
    let qh = usb_queuehead(CDC_RX_ENDPOINT as usize, false);
    let base_address = unsafe { RX_BUFFER.as_ptr() } as u32;
    let i = (packet.pointer0 - base_address) / RX_BUFFER_SIZE as u32;
    let len = (RX_BUFFER_SIZE as u32) - (packet.status >> 16) & 0x7FFF;
    let start = (i as usize) * RX_BUFFER_SIZE;
    let end = start + len as usize;

    // // Read the bytes
    for index in start..end {
        unsafe {
            BUFFER.enqueue(RX_BUFFER.bytes[index]);
        }
    }

    // If the queueheads have been reset, let's
    // re-initialize it all.
    //
    // IDK if this is a good idea??
    // What is the benefit of this vs. just having 1 descriptor?
    // I'm assuming multiple pages will give us some amount of
    // concurrency resilience but not really sure.
    if qh.first_transfer == qh.last_transfer && qh.first_transfer == 0 {
        // Process
        rx_queue_transfer();
    }
}

pub fn usb_serial_available() -> usize {
    return unsafe { BUFFER.size() };
}

pub fn usb_serial_read() -> Option<u8> {
    return unsafe { BUFFER.dequeue() };
}

pub fn usb_serial_peek() -> Option<u8> {
    unsafe {
        if BUFFER.size() > 0 {
            return Some(BUFFER.data[0]);
        } else {
            return None;
        }
    }
}

fn tx_callback(packet: &UsbEndpointTransferDescriptor) {
    if (packet.status & 0x80) == 0 {
        let qh = usb_queuehead(CDC_TX_ENDPOINT as usize, true);
        qh.first_transfer = 0;
        qh.last_transfer = 0;
        unsafe {
            TX_AVAILABLE = 0;
        }
    } else {
        usb_timer_oneshot();
    }
}

pub fn usb_serial_putchar(byte: u8) {
    usb_serial_write(&[byte]);
}

pub fn usb_serial_write(bytes: &[u8]) {
    unsafe {
        for byte in bytes {
            TX_BUFFER_TRANSIENT.enqueue(byte.clone());
        }
    }
    usb_timer_oneshot();
}

pub fn usb_serial_flush() {
    // Prepare
    if unsafe { TX_BUFFER_TRANSIENT.size() } > 0 {
        // Tail of RX
        let mut page = 0;
        for i in 0..TX_COUNT {
            let dtd = unsafe { &mut TX_DTD[i] };
            if (dtd.status.clone() & 0x80) == 0 {
                page = i;
                break;
            }
        }

        let dtd = unsafe { &mut TX_DTD[page] };
        if (dtd.status.clone() & 0x80) > 0 {
            usb_timer_oneshot();
            // Still active
            return;
        }

        debug_str(b"flush");

        unsafe {
            arm_dcache_delete(
                TX_BUFFER[page].as_ptr() as u32,
                TX_BUFFER_TRANSIENT.size() as u32,
            );
        }

        // Copy the data.
        for i in 0..unsafe { TX_BUFFER_TRANSIENT.size() } {
            unsafe {
                TX_BUFFER[page].bytes[i] = TX_BUFFER_TRANSIENT.data[i];
            }
        }

        usb_prepare_transfer(
            dtd,
            unsafe { TX_BUFFER[page].as_ptr() } as u32,
            unsafe { TX_BUFFER_TRANSIENT.size() } as u32,
            true,
        );

        unsafe {
            TX_AVAILABLE += TX_BUFFER_TRANSIENT.size();
            // Clear buffer
            TX_BUFFER_TRANSIENT.clear();
        }
        usb_transmit(CDC_TX_ENDPOINT as usize, dtd);
    }
}

fn setup_cdc_descriptors() {
    let descriptors = usb_get_descriptors();

    // High-speed interface descriptors
    descriptors.with_interface(
        0x200,
        0x0,
        &[
            // interface association descriptor, USB ECN, Table 9-Z
            8,                    // bLength
            11,                   // bDescriptorType
            CDC_STATUS_INTERFACE, // bFirstInterface
            2,                    // bInterfaceCount
            0x02,                 // bFunctionClass
            0x02,                 // bFunctionSubClass
            0x01,                 // bFunctionProtocol
            0,
            // configuration for 480 Mbit/sec speed
            // interface descriptor, USB spec 9.6.5, page 267-269, Table 9-12
            9,                    // bLength
            4,                    // bDescriptorType
            CDC_STATUS_INTERFACE, // bInterfaceNumber
            0,                    // bAlternateSetting
            1,                    // bNumEndpoints
            0x02,                 // bInterfaceClass
            0x02,                 // bInterfaceSubClass
            0x01,                 // bInterfaceProtocol
            0,                    // iInterface
            // CDC Header Functional Descriptor, CDC Spec 5.2.3.1, Table 26
            5,    // bFunctionLength
            0x24, // bDescriptorType
            0x00, // bDescriptorSubtype
            0x10,
            0x01, // bcdCDC
            // Call Management Functional Descriptor, CDC Spec 5.2.3.2, Table 27
            5,    // bFunctionLength
            0x24, // bDescriptorType
            0x01, // bDescriptorSubtype
            0x01, // bmCapabilities
            1,    // bDataInterface
            // Abstract Control Management Functional Descriptor, CDC Spec 5.2.3.3, Table 28
            4,    // bFunctionLength
            0x24, // bDescriptorType
            0x02, // bDescriptorSubtype
            0x06, // bmCapabilities
            // Union Functional Descriptor, CDC Spec 5.2.3.8, Table 33
            5,                    // bFunctionLength
            0x24,                 // bDescriptorType
            0x06,                 // bDescriptorSubtype
            CDC_STATUS_INTERFACE, // bMasterInterface
            CDC_DATA_INTERFACE,   // bSlaveInterface0
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,                       // bLength
            5,                       // bDescriptorType
            CDC_ACM_ENDPOINT | 0x80, // bEndpointAddress
            0x03,                    // bmAttributes (0x03=intr)
            lsb(CDC_ACM_SIZE),
            msb(CDC_ACM_SIZE), // wMaxPacketSize
            5,                 // bInterval
            // interface descriptor, USB spec 9.6.5, page 267-269, Table 9-12
            9,                  // bLength
            4,                  // bDescriptorType
            CDC_DATA_INTERFACE, // bInterfaceNumber
            0,                  // bAlternateSetting
            2,                  // bNumEndpoints
            0x0A,               // bInterfaceClass
            0x00,               // bInterfaceSubClass
            0x00,               // bInterfaceProtocol
            0,                  // iInterface
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,               // bLength
            5,               // bDescriptorType
            CDC_RX_ENDPOINT, // bEndpointAddress
            0x02,            // bmAttributes (0x02=bulk)
            lsb(CDC_RX_SIZE_480),
            msb(CDC_RX_SIZE_480), // wMaxPacketSize
            0,                    // bInterval
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,                      // bLength
            5,                      // bDescriptorType
            CDC_TX_ENDPOINT | 0x80, // bEndpointAddress
            0x02,                   // bmAttributes (0x02=bulk)
            lsb(CDC_TX_SIZE_480),
            msb(CDC_TX_SIZE_480), // wMaxPacketSize
            0,
        ],
    );

    // Low-speed interface descriptor
    descriptors.with_interface(
        0x700,
        0x0,
        &[
            // interface association descriptor, USB ECN, Table 9-Z
            8,                    // bLength
            11,                   // bDescriptorType
            CDC_STATUS_INTERFACE, // bFirstInterface
            2,                    // bInterfaceCount
            0x02,                 // bFunctionClass
            0x02,                 // bFunctionSubClass
            0x00,                 // bFunctionProtocol
            0,
            // configuration for 12 Mbit/sec speed
            // interface descriptor, USB spec 9.6.5, page 267-269, Table 9-12
            9,                    // bLength
            4,                    // bDescriptorType
            CDC_STATUS_INTERFACE, // bInterfaceNumber
            0,                    // bAlternateSetting
            1,                    // bNumEndpoints
            0x02,                 // bInterfaceClass
            0x02,                 // bInterfaceSubClass
            0x00,                 // bInterfaceProtocol
            0,                    // iInterface
            // CDC Header Functional Descriptor, CDC Spec 5.2.3.1, Table 26
            5,    // bFunctionLength
            0x24, // bDescriptorType
            0x00, // bDescriptorSubtype
            0x10,
            0x01, // bcdCDC
            // Call Management Functional Descriptor, CDC Spec 5.2.3.2, Table 27
            5,    // bFunctionLength
            0x24, // bDescriptorType
            0x01, // bDescriptorSubtype
            0x01, // bmCapabilities
            1,    // bDataInterface
            // Abstract Control Management Functional Descriptor, CDC Spec 5.2.3.3, Table 28
            4,    // bFunctionLength
            0x24, // bDescriptorType
            0x02, // bDescriptorSubtype
            0x06, // bmCapabilities
            // Union Functional Descriptor, CDC Spec 5.2.3.8, Table 33
            5,                    // bFunctionLength
            0x24,                 // bDescriptorType
            0x06,                 // bDescriptorSubtype
            CDC_STATUS_INTERFACE, // bMasterInterface
            CDC_DATA_INTERFACE,   // bSlaveInterface0
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,                       // bLength
            5,                       // bDescriptorType
            CDC_ACM_ENDPOINT | 0x80, // bEndpointAddress
            0x03,                    // bmAttributes (0x03=intr)
            CDC_ACM_SIZE as u8,
            0,  // wMaxPacketSize
            16, // bInterval
            // interface descriptor, USB spec 9.6.5, page 267-269, Table 9-12
            9,                  // bLength
            4,                  // bDescriptorType
            CDC_DATA_INTERFACE, // bInterfaceNumber
            0,                  // bAlternateSetting
            2,                  // bNumEndpoints
            0x0A,               // bInterfaceClass
            0x00,               // bInterfaceSubClass
            0x00,               // bInterfaceProtocol
            0,                  // iInterface
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,               // bLength
            5,               // bDescriptorType
            CDC_RX_ENDPOINT, // bEndpointAddress
            0x02,            // bmAttributes (0x02=bulk)
            lsb(CDC_RX_SIZE_12),
            msb(CDC_RX_SIZE_12), // wMaxPacketSize
            0,                   // bInterval
            // endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
            7,                      // bLength
            5,                      // bDescriptorType
            CDC_TX_ENDPOINT | 0x80, // bEndpointAddress
            0x02,                   // bmAttributes (0x02=bulk)
            lsb(CDC_TX_SIZE_12),
            msb(CDC_TX_SIZE_12), // wMaxPacketSize
            0,
        ],
    );
}
