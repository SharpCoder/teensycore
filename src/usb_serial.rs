use crate::{debug::debug_str, phys::usb::models::*, phys::usb::*};

const TX_ENDPOINT: usize = 2;
const RX_ENDPOINT: usize = 3;

pub fn usb_serial_init() {
    // Hook in the various callbacks
    usb_attach_setup_callback(usb_serial_configure);
}

fn usb_serial_configure(packet: SetupPacket) {
    match packet.bm_request_and_type {
        // SET_CONFIGURATION packet
        0x900 => {
            // Configure the endpoints.
            debug_str(b"configure endpoints from usb_serial");

            // Setup the serial transmit endpoint
            usb_setup_endpoint(
                TX_ENDPOINT,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::BULK,
                    size: 64,
                    callback: None,
                }),
                None,
            );

            // Setup the serial receive endpoint
            usb_setup_endpoint(
                RX_ENDPOINT,
                None,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::BULK,
                    size: 64,
                    callback: None,
                }),
            );
        }
        _ => {
            // Do nothing
            return;
        }
    }
}
