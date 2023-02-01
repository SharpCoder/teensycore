#![allow(dead_code)]

const MANUFACTURER_SIZE: usize = 8;
const MANUFACTURER_NAME: &[u8; MANUFACTURER_SIZE] = b"Debuggle";

const PRODUCT_SIZE: usize = 10;
const PRODUCT_NAME: &[u8; PRODUCT_SIZE] = b"Teensycore";
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

// How many descriptors are present in the system.
// Whenever you add one, sadly you have to increment this
// manually
const CONFIG_DESC_SIZE: usize = 8;
pub const CONFIG_DESC_BYTES: usize =
    18 + 10 + 9 + 2 + 2 + 2 + MANUFACTURER_SIZE * 2 + 2 + PRODUCT_SIZE * 2;
const NUM_INTERFACE: u8 = 2;

// No idea where this comes from but would
// like to try changing it once this works.
const PRODUCT_ID: u16 = 0x483;

// No idea where this comes from
const VENDOR_ID: u16 = 0x16C0;

pub const DESCRIPTOR_LIST: [Descriptor; CONFIG_DESC_SIZE] = [
    Descriptor {
        w_value: 0x100,
        w_index: 0x0,
        payload: DescriptorPayload::Device([
            18,              // bLength
            1,               // bDescriptorType
            0x0,             // bcdUSB LSB
            0x2,             // bcdUSB MSB
            0,               // bDeviceClass
            0,               // bDeviceSubClass
            0,               // bDeviceProtocol
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
            0,   // bDeviceSubClass
            0,   // bDeviceProtocol
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
            9,                             // bLength
            2,                             // bDescriptorType
            lsb(CONFIG_DESC_BYTES as u16), // wTotalLength
            msb(CONFIG_DESC_BYTES as u16), // wTotalLength
            NUM_INTERFACE,                 // bNumInterfaces
            1,                             // bConfigurationValue
            0,                             // iConfiguration
            0xC0,                          // bmAttributes
            50,                            // bMaxPower
        ]),
    },
    // Low-Speed
    Descriptor {
        w_value: 0x700,
        w_index: 0x0,
        payload: DescriptorPayload::Config([
            9,                             // bLength
            2,                             // bDescriptorType
            lsb(CONFIG_DESC_BYTES as u16), // wTotalLength
            msb(CONFIG_DESC_BYTES as u16), // wTotalLength
            NUM_INTERFACE,                 // bNumInterfaces
            1,                             // bConfigurationValue
            0,                             // iConfiguration
            0xC0,                          // bmAttributes
            50,                            // bMaxPower
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
        payload: DescriptorPayload::String(PRODUCT_NAME),
    },
];

const fn msb(val: u16) -> u8 {
    return (val >> 8) as u8;
}

const fn lsb(val: u16) -> u8 {
    return (val & 0xFF) as u8;
}
