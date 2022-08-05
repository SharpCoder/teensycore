use crate::phys::usb::*;
use crate::debug::*;

pub fn usb_serial_init() {

    debug_str(b"[usb] Initializing endpoints");
    usb_set_mode(UsbMode::DEVICE);
    usb_initialize_endpoints();
    
    // Configure
    // usb_configure_endpoint(1, UsbEndpointConfig {
    //     stall: false,
    //     reset: true,
    //     enabled: true,
    //     endpoint_type: UsbEndpointType::INTERRUPT,
    // });
    
    usb_irq_enable(USBINT | USBERRINT | URI);
    debug_str(b"[usb] enabled irq");
    usb_restart();
    debug_str(b"[usb] set start/run bit");
}