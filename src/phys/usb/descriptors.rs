#![allow(dead_code)]

use crate::system::vector::Array;
use crate::system::vector::Stack;
use crate::system::vector::Vector;

const MANUFACTURER_NAME: &[u8] = b"SharpCoder";
const PRODUCT_NAME: &[u8] = b"Teensycore";
const SERIAL_NUMBER: &[u8] = b"1337";

#[derive(Copy, Clone)]
pub struct Descriptor2 {
    pub w_value: u16,
    pub w_index: u16,
    pub payload: Vector<u8>,
}

pub struct Descriptors {
    pub vid: u16,
    pub pid: u16,
    pub descriptor_list: Vector<Descriptor2>,
    pub class_specific_interfaces: Vector<Descriptor2>,
}

impl Descriptors {
    pub const fn new() -> Self {
        return Descriptors {
            vid: 0x1209,
            pid: 0xF314,
            descriptor_list: Vector::new(),
            class_specific_interfaces: Vector::new(),
        };
    }

    pub fn clear(&mut self) {
        self.descriptor_list.clear();
        self.class_specific_interfaces.clear();
    }

    pub fn with_descriptor(&mut self, w_value: u16, w_index: u16, payload: &[u8]) {
        self.descriptor_list.push(Descriptor2 {
            w_value: w_value,
            w_index: w_index,
            payload: Vector::from_slice(payload),
        });
    }

    pub fn with_string(&mut self, w_value: u16, w_index: u16, bytes: &[u8]) {
        let mut vec: Vector<u8> = Vector::from_slice(&[2 + (bytes.len() as u8) * 2, 3]);
        for byte in bytes {
            vec.push(*byte);
            vec.push(0);
        }

        self.descriptor_list.push(Descriptor2 {
            w_value: w_value,
            w_index: w_index,
            payload: vec.clone(),
        });
    }

    pub fn with_interface(&mut self, w_value: u16, w_index: u16, payload: &[u8]) {
        self.class_specific_interfaces.push(Descriptor2 {
            w_value: w_value,
            w_index: w_index,
            payload: Vector::from_slice(payload),
        });
    }

    pub fn set_codes(&mut self, vid: u16, pid: u16) {
        self.vid = vid;
        self.pid = pid;
    }

    pub fn get_bytes(&self, w_value: u16, w_index: u16) -> Option<Vector<u8>> {
        for descriptor in self.descriptor_list.into_iter() {
            if descriptor.w_value == w_value && descriptor.w_index == w_index {
                let mut bytes = descriptor.payload.clone();

                // Now check if it's a config, because if so, we need to
                // send the class specific interfaces with it.
                for interface in self.class_specific_interfaces.into_iter() {
                    if interface.w_value == w_value && interface.w_index == w_index {
                        bytes.join(&interface.payload);
                    }
                }

                // Override the VendorID and ProductID
                if w_value == 0x100 && w_index == 0x00 {
                    bytes.put(8, lsb(self.vid));
                    bytes.put(9, msb(self.vid));
                    bytes.put(10, lsb(self.pid));
                    bytes.put(11, msb(self.pid));
                }

                // Config type
                if w_value == 0x200 || w_value == 0x700 {
                    // Update the specific bytes that describe the size of the interface
                    bytes.put(2, lsb(bytes.size() as u16));
                    bytes.put(3, msb(bytes.size() as u16));
                    bytes.put(4, self.class_specific_interfaces.size() as u8);
                }

                return Some(bytes);
            }
        }

        return None;
    }
}

static mut DESCRIPTORS: Descriptors = Descriptors::new();

pub fn usb_get_descriptors() -> &'static mut Descriptors {
    return unsafe { &mut DESCRIPTORS };
}

pub fn usb_initialize_descriptors() {
    let descriptors = usb_get_descriptors();
    descriptors.clear();

    // Device desciptor
    descriptors.with_descriptor(
        0x100,
        0x0,
        &[
            18,   // bLength
            1,    // bDescriptorType
            0x0,  // bcdUSB lsb
            0x02, // bcdUSB msb
            2,    // bDeviceClass (2 = Communication)
            2,    // bDeviceSubClass
            1,    // bDeviceProtocol
            64,   // bMaxPacketSize0
            0,    // VendorID (injected with code)
            0,    // VendorID (injected with code)
            0,    // ProductID (injected with code)
            0,    // ProductID (injected with code)
            0x79, // bcdDevice
            0x02, // bcdDevice
            1,    // iManufacturer (Index of string descriptor describing manufacturer)
            2,    // iProduct
            3,    // iSerialNumber
            1,    // bNumConfigurations
        ],
    );

    // Qualifier
    descriptors.with_descriptor(
        0x600,
        0x0,
        &[
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
        ],
    );

    // High-speed configuration
    descriptors.with_descriptor(
        0x200,
        0x0,
        &[
            9,    // bLength
            2,    // bDescriptorType
            9,    // wTotalLength (lsb)
            0,    // wTotalLength (msb)
            0,    // bNumInterfaces (computed)
            1,    // bConfigurationValue
            0,    // iConfiguration
            0xC0, // bmAttributes
            50,
        ],
    );

    // Low-speed configuration
    descriptors.with_descriptor(
        0x700,
        0x0,
        &[
            9,    // bLength
            2,    // bDescriptorType
            9,    // wTotalLength (lsb)
            0,    // wTotalLength (msb)
            0,    // bNumInterfaces (computed)
            1,    // bConfigurationValue
            0,    // iConfiguration
            0xC0, // bmAttributes
            50,
        ],
    );

    // Language codes (American English)
    descriptors.with_descriptor(0x300, 0x0, &[4, 3, lsb(0x409), msb(0x409)]);

    // Strings
    descriptors.with_string(0x301, 0x409, MANUFACTURER_NAME);
    descriptors.with_string(0x302, 0x409, PRODUCT_NAME);
    descriptors.with_string(0x303, 0x409, SERIAL_NUMBER);
}

pub const fn msb(val: u16) -> u8 {
    return (val >> 8) as u8;
}

pub const fn lsb(val: u16) -> u8 {
    return (val & 0xFF) as u8;
}
