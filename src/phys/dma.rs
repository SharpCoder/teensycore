#![allow(dead_code)]

use crate::phys::*;
use crate::phys::addrs;

const TCR_CSR: u32 = 0x101C;
const TCD_SADDR: u32 = 0x1000;
const TCD_SOFF: u32 = 0x1004;
const TCD_SATTR: u32 = 0x1006;
const TCD_NBYTES: u32 = 0x1008;
const TCD_SLAST: u32 = 0x100C;
const TCD_CITER: u32 = 0x1016;
const TCD_BITER: u32 = 0x101E;
const TCD_DADDR: u32 = 0x1010;
const TCD_DLASTSGA: u32 = 0x1018;
const TCD_DOFF: u32 = 0x1014;

pub enum DMASource {
    Uart1Tx = 2,
    Uart1Rx = 3,
    Uart3Tx = 4,
    Uart3Rx = 5,
    Uart5Tx = 6,
    Uart5Rx = 7,
    Uart7Tx = 8,
    Uart7Rx = 9,
    Uart2Tx = 66,
    Uart2Rx = 67,
    Uart4Tx = 68,
    Uart4Rx = 69,
    Uart6Tx = 70,
    Uart6Rx = 71,
    Uart8Tx = 72,
    Uart8Rx = 73,
}

type DMAChannel = u32;

fn get_addr(channel: DMAChannel) -> u32 {
    return addrs::DMAMUX + (channel * 4);
}

pub fn dma_start_clock() {
    assign(0x400F_C07C, read_word(0x400F_C07C) | (0x3 << 6));
}

pub fn dma_enable(channel: DMAChannel) {
    // Enable DMA
    let addr = get_addr(channel);
    assign(addr, read_word(addr) | (0x1 << 31));
}

pub fn dma_get_errors() -> u32 {
    return read_word(addrs::DMA + 0x4);
}

pub fn dma_is_irq(channel: DMAChannel) -> bool {
    return read_word(addrs::DMA + 0x24) & (0x1 << channel) > 0;
}

pub fn dma_enable_irq(channel: DMAChannel) {
    let origin = read_word(addrs::DMA + 0x24);
    assign(addrs::DMA + 0x24, origin | (0x1 << channel));
}

pub fn dma_enable_request(channel: DMAChannel) {
    assign_8(addrs::DMA + 0x1B, (channel + 1) as u8);
}

pub fn dma_disable_on_completion(channel: DMAChannel) {
    let addr = addrs::DMA + TCR_CSR + (channel * 0x20);
    let csr = read_word(addr);
    assign(addr, csr | (0x1 << 3));

}

pub fn dma_clear_irq(channel: DMAChannel) {
    assign_8(addrs::DMA + 0x1F, (channel + 1) as u8);
}

pub fn dma_interrupt_at_completion(channel: DMAChannel) {
    let addr = addrs::DMA + TCR_CSR + (channel * 0x20);
    let csr = read_word(addr);
    assign(addr, csr | 0x2);
}

pub fn dma_disable_request(channel: DMAChannel) {
    assign_8(addrs::DMA + 0x1A, (channel + 1) as u8);
}

pub fn dma_clear_done_status(channel: DMAChannel) {
    assign_8(addrs::DMA + 0x1C, (channel + 1) as u8);
}

pub fn dma_disable(channel: DMAChannel) {
    let addr = get_addr(channel);
    assign(addr, read_word(addr) & !(0x1 << 31));
}

pub fn dma_trigger_enable(channel: DMAChannel) {
    let addr = get_addr(channel);
    assign(addr, read_word(addr) | (0x1 << 30));
}

pub fn dma_trigger_disable(channel: DMAChannel) {
    let addr = get_addr(channel);
    assign(addr, read_word(addr) & !(0x1 << 30));
}

pub fn dma_configure_source(channel: DMAChannel, source: DMASource) {
    let addr = get_addr(channel);
    assign(addr, read_word(addr) & !(0x3F) | (source as u32));
}

// Meant to be used with [u8] buffer
pub fn dma_source_buffer(channel: DMAChannel, buffer: u32, length: u16) {
    assign(addrs::DMA + TCD_SADDR + (channel * 0x20), buffer);
    assign_16(addrs::DMA + TCD_SOFF + (channel * 0x20), 0x1);
    assign_16(addrs::DMA + TCD_SATTR + (channel * 0x20), read_word(addrs::DMA + TCD_SATTR + (channel * 0x20)) as u16 & !(0x3 << 8));
    assign(addrs::DMA + TCD_NBYTES + (channel * 0x20), 0x01);
    // Is this right?
    assign(addrs::DMA + TCD_SLAST + (channel * 0x20), 0xFFFF_FFFF - length as u32);
    assign_16(addrs::DMA + TCD_CITER + (channel * 0x20), length);
    assign_16(addrs::DMA + TCD_BITER + (channel * 0x20), length);
    dma_enable_request(channel);
}

// Meant to be used with [u8] buffer
pub fn dma_dest_buffer(channel: DMAChannel, buffer: u32, length: u16) {
    assign(addrs::DMA + TCD_DADDR + (channel * 0x20), buffer);
    assign_16(addrs::DMA + TCD_DOFF + (channel * 0x20), 0x1);
    assign_16(addrs::DMA + TCD_SATTR + (channel * 0x20),read_word(addrs::DMA + TCD_SATTR + (channel * 0x20)) as u16 & !0x3);
    assign(addrs::DMA + TCD_NBYTES + (channel * 0x20), 0x01);

    // Is this right?
    assign(addrs::DMA + TCD_DLASTSGA + (channel * 0x20), 0xFFFF_FFFF - length as u32);
    assign_16(addrs::DMA + TCD_CITER + (channel * 0x20), length);
    assign_16(addrs::DMA + TCD_BITER + (channel * 0x20), length);
    dma_enable_request(channel);
}

pub fn dma_source_addr(channel: DMAChannel, source: u32) {
    assign(addrs::DMA + TCD_SADDR + (channel * 0x20), source);
    assign(addrs::DMA + TCD_SOFF + (channel * 0x20), 0x0);
    assign_16(addrs::DMA + TCD_SATTR + (channel * 0x20), 0x2);

    let n_bytes = read_word(addrs::DMA + TCD_NBYTES);
    if source < 0x40000000 || n_bytes == 0 {
        assign(addrs::DMA + TCD_NBYTES, 0x4);
    }

    assign(addrs::DMA + TCD_SLAST + (channel * 0x20), 0x0);
}

pub fn dma_dest_addr(channel: DMAChannel, destination: u32) {
    assign(addrs::DMA + TCD_DADDR + (channel * 0x20), destination);
    assign_16(addrs::DMA + TCD_DOFF + (channel * 0x20), 0x00); // Signed offset 
    assign(addrs::DMA + TCD_DLASTSGA + (channel * 0x20), 0x00); // TCD Last Destination Address Adjustment/Scatter Gather Address

    let n_bytes = read_word(addrs::DMA + TCD_NBYTES);
    if destination < 0x40000000 || n_bytes == 0 {
        assign(addrs::DMA + TCD_NBYTES, 0x1);
    }

    // Read csr
    let csr = read_word(addrs::DMA + TCR_CSR + (channel * 0x20));
    assign(addrs::DMA + TCR_CSR + (channel * 0x20), csr | 0x03);
}