#![allow(dead_code)]

const MANUFACTURER_NAME: &[u8] = b"Debuggle";
const PRODUCT_NAME: &[u8] = b"Teensycore";
const SERIAL_NUMBER: &[u8] = b"1337";

pub enum DescriptorPayload {
    Device([u8; 18]),
    Qualifier([u8; 10]),
    Config([u8; 9]),
    SupportedLanguages(&'static [u16]),
    String(&'static [u8]),
}

pub struct Descriptor {
    pub w_value: u32,
    pub w_index: u32,
    pub payload: DescriptorPayload,
}

pub const HS_CONFIG_DESC_BYTES: usize = 9 + HIGH_SPEED_INTERFACE_DESCRIPTOR.len();
pub const LS_CONFIG_DESC_BYTES: usize = 9 + LOW_SPEED_INTERFACE_DESCRIPTOR.len();
const NUM_INTERFACE: u8 = 2;
const CDC_STATUS_INTERFACE: u8 = 0;
const CDC_DATA_INTERFACE: u8 = 1;
const CDC_ACM_ENDPOINT: u8 = 2;
const CDC_ACM_SIZE: u16 = 16;
const CDC_RX_ENDPOINT: u8 = 3;
const CDC_TX_ENDPOINT: u8 = 4;
const CDC_RX_SIZE_480: u16 = 512;
const CDC_TX_SIZE_480: u16 = 512;
const CDC_RX_SIZE_12: u16 = 64;
const CDC_TX_SIZE_12: u16 = 64;

const PRODUCT_ID: u16 = 0xBADD;
const VENDOR_ID: u16 = 0x1337;

pub const DESCRIPTOR_LIST: [Descriptor; 8] = [
    Descriptor {
        w_value: 0x100,
        w_index: 0x0,
        payload: DescriptorPayload::Device([
            18,              // bLength
            1,               // bDescriptorType
            0x0,             // bcdUSB lsb
            0x02,            // bcdUSB msb
            2,               // bDeviceClass (2 = Communication)
            2,               // bDeviceSubClass
            1,               // bDeviceProtocol
            64,              // bMaxPacketSize0
            lsb(VENDOR_ID),  // VendorID
            msb(VENDOR_ID),  // VendorID
            lsb(PRODUCT_ID), // ProductID
            msb(PRODUCT_ID), // ProductID
            0x79,            // bcdDevice
            0x02,            // bcdDevice
            1,               // iManufacturer (Index of string descriptor describing manufacturer)
            2,               // iProduct
            3,               // iSerialNumber
            1,               // bNumConfigurations
        ]),
    },
    Descriptor {
        w_value: 0x600,
        w_index: 0x0,
        payload: DescriptorPayload::Qualifier([
            10,  // bLength
            6,   // bDescriptorType
            0x0, // bcdUSB
            0x2, // bcdUSB
            2,   // bDeviceClass
            2,   // bDeviceSubClass
            1,   // bDeviceProtocol
            64,  // bMaxPacketSize0,
            1,   // bNumConfigurations
            0,   // bReserved
        ]),
    },
    // High-Speed
    Descriptor {
        w_value: 0x200,
        w_index: 0x0,
        payload: DescriptorPayload::Config([
            9,                                // bLength
            2,                                // bDescriptorType
            lsb(HS_CONFIG_DESC_BYTES as u16), // wTotalLength
            msb(HS_CONFIG_DESC_BYTES as u16), // wTotalLength
            NUM_INTERFACE,                    // bNumInterfaces
            1,                                // bConfigurationValue
            0,                                // iConfiguration
            0xC0,                             // bmAttributes
            50,                               // bMaxPower
        ]),
    },
    // Low-Speed
    Descriptor {
        w_value: 0x700,
        w_index: 0x0,
        payload: DescriptorPayload::Config([
            9,                                // bLength
            2,                                // bDescriptorType
            lsb(LS_CONFIG_DESC_BYTES as u16), // wTotalLength
            msb(LS_CONFIG_DESC_BYTES as u16), // wTotalLength
            NUM_INTERFACE,                    // bNumInterfaces
            1,                                // bConfigurationValue
            0,                                // iConfiguration
            0xC0,                             // bmAttributes
            50,                               // bMaxPower
        ]),
    },
    Descriptor {
        w_value: 0x300,
        w_index: 0x0,
        payload: DescriptorPayload::SupportedLanguages(&[
            // English (United States)
            0x409,
        ]),
    },
    // Manufacturer
    Descriptor {
        w_value: 0x301,
        w_index: 0x409,
        payload: DescriptorPayload::String(MANUFACTURER_NAME),
    },
    // Product Name
    Descriptor {
        w_value: 0x302,
        w_index: 0x409,
        payload: DescriptorPayload::String(PRODUCT_NAME),
    },
    // Serial number
    Descriptor {
        w_value: 0x303,
        w_index: 0x409,
        payload: DescriptorPayload::String(SERIAL_NUMBER),
    },
];

pub const HIGH_SPEED_INTERFACE_DESCRIPTOR: &[u8] = &[
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
];

pub const LOW_SPEED_INTERFACE_DESCRIPTOR: &[u8] = &[
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
];

const fn msb(val: u16) -> u8 {
    return (val >> 8) as u8;
}

const fn lsb(val: u16) -> u8 {
    return (val & 0xFF) as u8;
}
