use crate::{
    arm_dcache_delete, mem,
    phys::{addrs::USB, usb::models::*, usb::registers::*},
    phys::{assign, read_word, usb::descriptors::*, usb::*},
    system::{
        buffer::*,
        vector::{Queue, Stack},
    },
};

// How many pages of data we support
const RX_BUFFER_SIZE: usize = 512;
const TX_BUFFER_SIZE: usize = 512;

static mut BUFFER: Buffer<512, u8> = Buffer::new(0);

#[link_section = ".descriptors"]
static mut TX_DTD: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor::new();
#[link_section = ".descriptors"]
static mut RX_DTD: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor::new();
static mut TX_BUFFER_TRANSIENT: Buffer<TX_BUFFER_SIZE, u8> = Buffer::new(0);
#[link_section = ".dmabuffers"]
static mut RX_BUFFER: BufferPage = BufferPage::new();

#[link_section = ".dmabuffers"]
static mut TX_BUFFER: BufferPage = BufferPage::new();
static mut CONFIGURED: bool = false;

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
}

fn usb_timer_oneshot() {
    let timer_val = read_word(USB + 0x84) & 0xFFDFFF;
    if timer_val == 0 {
        assign(USB + 0x84, (1 << 31) | (1 << 30));
    }
}

fn handle_irq(irq_status: u32) {
    if (irq_status & TI0) > 0 {
        usb_serial_flush();
    }
}

fn usb_serial_configure(packet: SetupPacket) {
    match packet.bm_request_and_type {
        0x2221 => {
            // The device is now present? Seems like an ok indicator for configured.
            unsafe {
                CONFIGURED = true;
            }
        }
        // SET_CONFIGURATION packet
        0x900 => {
            // Configure the endpoints.
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
            unsafe {
                TX_DTD.clear();
                RX_DTD.clear();
            }

            rx_queue_transfer();

            // Configure timer
            assign(USB + 0x80, 0x0003E7); // 1ms 0x0003E7
            assign(USBINTR, read_word(USBINTR) | TI0);
            assign(USB + 0x84, (1 << 31) | (1 << 30));
        }
        _ => {
            // Do nothing
            return;
        }
    }
}

fn rx_queue_transfer() {
    let rx_buffer_len = RX_BUFFER_SIZE as u32;
    let buffer_ptr = unsafe { RX_BUFFER.as_ptr() as u32 };

    arm_dcache_delete(buffer_ptr, rx_buffer_len);
    usb_prepare_transfer(unsafe { &mut RX_DTD }, buffer_ptr, rx_buffer_len, true);
    usb_receive(CDC_RX_ENDPOINT as usize, unsafe { &mut RX_DTD });
}

fn rx_callback(packet: &UsbEndpointTransferDescriptor) {
    if unsafe { CONFIGURED } == false {
        return;
    }

    let qh = usb_get_queuehead(CDC_RX_ENDPOINT as usize, false);
    let len = (RX_BUFFER_SIZE as u32) - (packet.status >> 16) & 0x7FFF;

    // // Read the bytes
    for index in 0..len {
        unsafe {
            BUFFER.enqueue(RX_BUFFER.bytes[index as usize]);
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

/// Returns how many bytes are available to read from
/// the buffer.
pub fn usb_serial_available() -> usize {
    return unsafe { BUFFER.size() };
}

/// Returns the next available byte in the buffer
/// and consumes it. If there are no bytes available,
/// this method will return None.
pub fn usb_serial_read() -> Option<u8> {
    return unsafe { BUFFER.dequeue() };
}

/// Return's the next available byte in the buffer
/// without consuming it. If there are no bytes available,
/// this method will return None.
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
    if (packet.status & 0xFF) != 0 {
        usb_timer_oneshot();
    }
}

/// Write a single byte to the USB host.
///
/// ```no_run
/// use teensycore::usb_serial::*;
/// usb_serial_putchar('!');
/// ```
pub fn usb_serial_putchar(byte: u8) {
    usb_serial_write(&[byte]);
}

/// Write an array of bytes to the USB host.
///
/// ```no_run
/// use teensycore::usb_serial::*;
/// usb_serial_write(b"Hello, world!");
/// ```
pub fn usb_serial_write(bytes: &[u8]) {
    unsafe {
        for byte in bytes {
            TX_BUFFER_TRANSIENT.push(*byte);
        }
    }
    usb_timer_oneshot();
}

/// This method will flush the current queue of data to
/// the usb host. It is invoked automatically based
/// on interrupts, but you can manually flush too if
/// you'd like. It will short-cirucit if all the
/// transfer queues are still active or if the USB host
/// hasn't finished connecting.
///
/// Returns how many bytes were written.
pub fn usb_serial_flush() -> u32 {
    // Prepare
    if unsafe { TX_BUFFER_TRANSIENT.size() } > 0 {
        // Verify we are in a good, configured state.
        if unsafe { CONFIGURED == false } {
            return 0;
        }

        let dtd = unsafe { &mut TX_DTD };

        // Check if it's done
        if (dtd.status & 0x80) > 0 {
            return 0;
        }

        // Check for error condition
        if (dtd.status & 0xFF) > 0 {
            return 0;
        }

        // Copy the data.
        let len = unsafe { TX_BUFFER_TRANSIENT.size() } as u32;
        let src_ptr = unsafe { TX_BUFFER_TRANSIENT.data.as_ptr() } as u32;
        let dst_ptr = unsafe { TX_BUFFER.as_ptr() } as u32;

        mem::copy(src_ptr, dst_ptr, len);

        unsafe {
            // Clear buffer
            TX_BUFFER_TRANSIENT.clear();
        }

        usb_prepare_transfer(dtd, dst_ptr, len, true);
        usb_transmit(CDC_TX_ENDPOINT as usize, dtd);
        return len;
    }

    return 0;
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
