use crate::{
    arm_dcache_delete,
    debug::debug_str,
    phys::{irq::Irq, usb::*},
    phys::{
        irq::{irq_disable, irq_enable},
        usb::models::*,
    },
};

static mut TX_TRANSFER: [UsbEndpointTransferDescriptor; 1] = [UsbEndpointTransferDescriptor::new()];
static mut RX_TRANSFER: [UsbEndpointTransferDescriptor; 1] = [UsbEndpointTransferDescriptor::new()];

#[link_section = ".dmabuffers"]
static mut RX_BUFFER: [u8; 64] = [0; 64];

const ACM_ENDPOINT: usize = 2;
const RX_ENDPOINT: usize = 3;
const TX_ENDPOINT: usize = 4;
const CDC_ACM_SIZE: u32 = 16;

pub fn usb_serial_init() {
    // Hook in the various callbacks
    usb_attach_setup_callback(usb_serial_configure);

    // Attach irq handler
    usb_attach_irq_handler(handle_irq);

    // // Configure timer
    // assign(USB + 0x80, 0x0003E7); // 1ms
    // assign(USB + 0x84, (1 << 31) | (1 << 24) | 0x0003E7);
}

fn handle_irq() {}

fn usb_serial_configure(packet: SetupPacket) {
    match packet.bm_request_and_type {
        // SET_CONFIGURATION packet
        0x900 => {
            // Configure the endpoints.
            debug_str(b"configure endpoints from usb_serial");

            usb_setup_endpoint(
                ACM_ENDPOINT,
                Some(EndpointConfig {
                    endpoint_type: EndpointType::INTERRUPT,
                    size: CDC_ACM_SIZE,
                    callback: None,
                }),
                None,
            );

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
        unsafe { &mut RX_TRANSFER[0] },
        unsafe { RX_BUFFER.as_ptr() },
        rx_buffer_len,
    );
    usb_receive(RX_ENDPOINT, unsafe { &mut RX_TRANSFER[0] });
    irq_enable(Irq::Usb1);
}

fn rx_callback() {
    debug_str(b"receive callback triggered");
}
