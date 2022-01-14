/***
 * On the Teensy 4.0, it has 3 periodic timers. The source clock is 100MHz (maybe)
**/
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
    unsafe {
        asm!("nop");
        crate::dsb();
    }
    pit_configure(source, PITConfig {
        chained: chained,
        irq_en: irq_en,
        en: true,
    });
    unsafe {
        asm!("nop");
        crate::dsb();
    }

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

pub fn pit_read_lifetime() -> u64 {
    let lifetime_high: u64 = read_word(addrs::PIT + 0xE0) as u64;
    let lifetime_low: u64 = read_word(addrs::PIT + 0xE4) as u64;

    return 0xFFFF_FFFF_FFFF_FFFF - ((lifetime_high << 32) + lifetime_low);
}