use crate::*;
use crate::{phys::usb::*, serio, system::str::*};

pub fn usb_serial_init() {
    serio::serial_write_str(serio::SerioDevice::Default, &str!(b"HI"));

    usb_set_mode(UsbMode::DEVICE);
    usb_initialize_endpoints();
    
    // Configure
    usb_configure_endpoint(0, UsbEndpointDirection::RX, UsbEndpointConfig {
        stall: false,
        enabled: true,
        reset: false,
        endpoint_type: UsbEndpointType::CONTROL,
    });

    usb_configure_endpoint(1, UsbEndpointDirection::TX, UsbEndpointConfig {
        stall: false,
        enabled: true,
        reset: false,
        endpoint_type: UsbEndpointType::CONTROL,
    });
    
    usb_irq_enable(USBINT | USBERRINT | PCI | URI);
    usb_start();
}