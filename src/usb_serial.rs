use crate::{phys::{usb::*, assign}, debug::blink_hardware};

pub fn usb_serial_init() {
    usb_set_mode(UsbMode::DEVICE);
    usb_initialize_endpoints();
    
    // Configure
    usb_configure_endpoint(0, UsbEndpointDirection::RX, UsbEndpointConfig {
        stall: false,
        enabled: true,
        reset: true,
        endpoint_type: UsbEndpointType::CONTROL,
    });

    usb_configure_endpoint(1, UsbEndpointDirection::TX, UsbEndpointConfig {
        stall: false,
        enabled: true,
        reset: true,
        endpoint_type: UsbEndpointType::CONTROL,
    });
    
    usb_irq_enable();
    usb_start();
}