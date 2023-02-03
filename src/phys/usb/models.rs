pub type Fn = fn();
pub type ConfigFn = fn(packet: SetupPacket);

pub enum UsbMode {
    DEVICE,
}

pub enum EndpointType {
    ISOCHRONOUS,
    BULK,
    INTERRUPT,
}

pub struct EndpointConfig {
    pub endpoint_type: EndpointType,
    pub size: u32,
    pub callback: Option<Fn>,
}

#[derive(Clone, Copy)]
pub struct SetupPacket {
    pub bm_request_and_type: u16,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
}

impl SetupPacket {
    pub fn from_dwords(word1: u32, word2: u32) -> SetupPacket {
        return SetupPacket {
            bm_request_and_type: (word1 & 0xFFFF) as u16,
            w_value: (word1 >> 16) as u16,
            w_index: (word2 & 0xFFFF) as u16,
            w_length: (word2 >> 16) as u16,
        };
    }
}

#[repr(C, align(64))]
pub struct UsbEndpointQueueHead {
    pub config: u32,
    pub current: u32,
    pub next: u32,
    pub status: u32,
    pub pointer0: u32,
    pub pointer1: u32,
    pub pointer2: u32,
    pub pointer3: u32,
    pub pointer4: u32,
    pub reserved: u32,
    pub setup0: u32,
    pub setup1: u32,
    pub callback: Fn,
}

#[repr(C, align(32))]
pub struct UsbEndpointTransferDescriptor {
    pub next: u32,
    pub status: u32,
    pub pointer0: u32,
    pub pointer1: u32,
    pub pointer2: u32,
    pub pointer3: u32,
    pub pointer4: u32,
    pub callback: Fn,
}
