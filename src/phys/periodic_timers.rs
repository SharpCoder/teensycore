//! This module provides access to the periodic timer peripheral.
//! 
//! The Teensy-4.0 has 4 individual periodic timers. It is important
//! to note however that the kernel itself allocates Timer 0 and Timer 1
//! for itself, to keep track of uptime.
//! 
//! System defaults configure the periodic timer to use the IPG clock
//! which, in normal system usage, is 132MHz - or - 7.5ns per clock cycle.
//! 
//! 
//! Periodic timers are capable of counting a specific number of clock
//! cycles and then issuing an interrupt.
//! 
//! Here is an example of using it:
//! 
//! ```no_run
//! use teensycore::debug::*;
//! use teensycore::phys::periodic_timers::*;
//! use teensycore::phys::irq::*;
//! 
//! pit_configure(&PeriodicTimerSource::Timer2, PITConfig {
//!     chained: false,
//!     irq_en: true,
//!     en: false,
//! });
//! 
//! pit_load_value(&PeriodicTimerSource::Timer2, 0x7F2_8155);
//! pit_restart(&PeriodicTimerSource::Timer2);
//! 
//! irq_attach(Irq::PeriodicTimer, handle_pit_irq);
//! irq_enable(Irq::PeriodicTimer);
//! 
//! 
//! fn handle_pit_irq() {
//!     debug_str(b"ping pong!");
//! }
//! ```
use core::arch::asm;

use crate::phys::addrs;
use crate::phys::{
    assign,
    read_word,
};

#[derive(Copy, Clone)]
pub enum PeriodicTimerSource {
    Timer0,
    Timer1,
    Timer2,
    Timer3,
}

pub struct PITConfig {
    pub chained: bool,
    pub irq_en: bool,
    pub en: bool,
}

fn pit_config_addr(source: &PeriodicTimerSource) -> u32 {
    return addrs::PIT + match source {
        PeriodicTimerSource::Timer0 => 0x108,
        PeriodicTimerSource::Timer1 => 0x118,
        PeriodicTimerSource::Timer2 => 0x128,
        PeriodicTimerSource::Timer3 => 0x138,
    };
}

pub fn pit_configure(source: &PeriodicTimerSource, config: PITConfig) {
    let mut value: u32 = 0x00;
    if config.chained {
        value |= 0x1 << 2;
    } 

    if config.irq_en {
        value |= 0x1 << 1;
    }

    if config.en {
        value |= 0x1;
    }
    
    let addr = pit_config_addr(&source);
    assign(addr, value);
}

pub fn pit_start_clock() {
    assign(0x400F_C01C, read_word(0x400F_C01C) & !0x7F);
}

pub fn pit_restart(source: &PeriodicTimerSource) {
    let original = read_word(pit_config_addr(&source));
    let chained = (original & 0x4) > 0;
    let irq_en = (original & 0x2) > 0;

    pit_configure(source, PITConfig {
        chained: chained,
        irq_en: irq_en,
        en: false,
    });
    crate::assembly!("nop");
    crate::dsb();
    pit_configure(source, PITConfig {
        chained: chained,
        irq_en: irq_en,
        en: true,
    });
    crate::assembly!("nop");
    crate::dsb();
}

/** This method starts the clock source generation */
pub fn pit_enable_clock() {
    assign(addrs::PIT, 0x1);
}

/** This method stops the clock source generation */
pub fn pit_disable_clock() {
    assign(addrs::PIT, 0x2);
}

pub fn pit_clear_interrupts(source: &PeriodicTimerSource) {
    let addr = addrs::PIT + match source {
        PeriodicTimerSource::Timer0 => 0x10C,
        PeriodicTimerSource::Timer1 => 0x11C,
        PeriodicTimerSource::Timer2 => 0x12C,
        PeriodicTimerSource::Timer3 => 0x13C,
    };

    assign(addr, 0x1);
}

pub fn pit_load_value(source: &PeriodicTimerSource, value: u32) {
    let addr = addrs::PIT + match source {
        PeriodicTimerSource::Timer0 => 0x100,
        PeriodicTimerSource::Timer1 => 0x110,
        PeriodicTimerSource::Timer2 => 0x120,
        PeriodicTimerSource::Timer3 => 0x130,
    };
    assign(addr, value);
}

/// Read how many clock cycles have occured since the system was turned on.
pub fn pit_read_lifetime() -> u64 {
    let lifetime_high: u64 = read_word(addrs::PIT + 0xE0) as u64;
    let lifetime_low: u64 = read_word(addrs::PIT + 0xE4) as u64;

    return 0xFFFF_FFFF_FFFF_FFFF - ((lifetime_high << 32) + lifetime_low);
}