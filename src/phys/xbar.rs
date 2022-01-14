use crate::phys::*;
use crate::phys::addrs;

pub fn xbar_start_clock() {
    assign(addrs::CCM_CCGR2, read_word(addrs::CCM_CCGR2) | (0x3 << 14) | (0x3 << 22) | (0x3 << 24));
}

pub fn xbar_connect(input: u32, output: u32) {
    let addr = addrs::IMXRT_XBARA1 + (output / 2);
    let val = read_16(addr);

    if !(output & 0x1) >= 1 {
        assign_16(addr, (val & 0xFF00) | (input as u16));
    } else {
        assign_16(addr, (val & 0x00FF) | ((input as u16) << 8));
    }
}