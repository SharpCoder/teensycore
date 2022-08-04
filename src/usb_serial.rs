use crate::phys::usb::*;

pub fn usb_serial_init() {
    usb_set_mode(UsbMode::DEVICE);
    usb_initialize_endpoints();
    usb_irq_enable();
    usb_start();
}