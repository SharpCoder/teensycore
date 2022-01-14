use crate::phys::{
    assign,
    read_word,
    set_bit,
    clear_bit,
};

use crate::phys::addrs::{
    GPT1,
    GPT2,
};

pub enum TimerSource {
    GPT1,
    GPT2,
}

pub enum TimerClock {
    None,
    Peripheral,
    HighFrequency,
    External,
    LowFrequency,
    Oscillator,
}

fn get_addr(timer: &TimerSource) -> u32 {
    return match timer {
        TimerSource::GPT1 => GPT1,
        TimerSource::GPT2 => GPT2,
    };
}

pub fn timer_enable(timer: &TimerSource) {
    let addr = get_addr(&timer);
    assign(addr, set_bit(read_word(addr), 0));
}

pub fn timer_enable_irq(timer: &TimerSource) {
    assign(get_addr(&timer) + 0xC, 0x23);
}

pub fn timer_disable_irq(timer: &TimerSource) {
    assign(get_addr(&timer) + 0xC, 0x0);
}

pub fn timer_clear_status(timer: &TimerSource) {
    assign(get_addr(&timer) + 0x8, 0x1F);
}

pub fn timer_assert_reset(timer: &TimerSource) {
    let addr = get_addr(&timer);
    assign(addr, read_word(addr) & (0x1 << 15));
}

pub fn timer_disable(timer: &TimerSource) {
    let addr = get_addr(&timer);
    assign(addr, clear_bit(read_word(addr), 0));
}

pub fn timer_read(timer: &TimerSource) -> u32 {
    return read_word(get_addr(&timer) + 0x24);
}

pub fn timer_set_clock(timer: &TimerSource, clock: TimerClock) {
    let addr = get_addr(&timer);
    let original_value = read_word(addr);
    let modifier = match clock {
        TimerClock::None => 0x0,
        TimerClock::Peripheral => 0x1,
        TimerClock::HighFrequency => 0x2,
        TimerClock::External => 0x3,
        TimerClock::LowFrequency => 0x4,
        TimerClock::Oscillator => 0x5,
    };

    // Take the original value, zero out the clock sequence from it, then override with new clock settings
    let clear_clock = 0x7 << 6;
    let next_value = (original_value & !clear_clock) | (modifier << 6) | 0x400;
    assign(addr + 0x4, 0x00); // Turn off all prescalers
    assign(addr, next_value);
}

pub fn timer_set_compare_value(timer: &TimerSource, compare_target: u32) {
    assign(get_addr(&timer) + 0x10, compare_target);
}