use crate::{
    arm_dcache_delete,
    debug::debug_str,
    phys::{
        addrs::USB,
        irq::{irq_disable, irq_enable},
        usb::models::*,
    },
    phys::{assign, irq::Irq, usb::descriptors::*, usb::*},
};

static mut RX_TRANSFER: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor::new();
static mut TX_TRANSFER: UsbEndpointTransferDescriptor = UsbEndpointTransferDescriptor::new();

#[link_section = ".dmabuffers"]
static mut RX_BUFFER: [u8; 512] = [0; 512];

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

    // // Configure timer
    if false {
        assign(USB + 0x80, 0x0003E7); // 1ms
        assign(USB + 0x84, (1 << 31) | (1 << 24) | 0x0003E7);
    }
}

fn handle_irq() {}

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
                    callback: None,
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
                    callback: Some(rx_callback),
                }),
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
    let rx_buffer_len = unsafe { RX_BUFFER.len() } as u32;
    arm_dcache_delete(unsafe { RX_BUFFER.as_ptr() } as u32, rx_buffer_len);
    usb_prepare_transfer(
        unsafe { &mut RX_TRANSFER },
        unsafe { RX_BUFFER.as_ptr() },
        rx_buffer_len,
    );
    usb_receive(CDC_RX_ENDPOINT as usize, unsafe { &mut RX_TRANSFER });
    irq_enable(Irq::Usb1);
}

fn rx_callback(packet: &UsbEndpointTransferDescriptor) {
    // blink_hardware(100);
    let len = (unsafe { RX_BUFFER.len() } as u32) - (packet.status >> 16) & 0x7FFF;
    // // Read the bytes
    debug_str(unsafe { &RX_BUFFER[0..(len as usize)] });
    // Queue a new receive packet.
    rx_queue_transfer();
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
