/**
 *  This is a system device which keeps track of time by using the periodic timer 
 **/
use crate::phys::*;
use crate::phys::periodic_timers::*;

pub fn clock_init() {
    // Setup clock
    periodic_timers::pit_start_clock();
    
    // // Undo clock gating
    assign(addrs::CCM_CCGR1, read_word(addrs::CCM_CCGR1) | (0x3 << 12));
    
    // Set CTRL 0
    pit_configure(&PeriodicTimerSource::Timer1, PITConfig {
        chained: true,
        irq_en: false,
        en: false,
    });

    // Configure timer 0
    pit_configure(&PeriodicTimerSource::Timer0, PITConfig {
        chained: false,
        irq_en: false,
        en: false,
    });

    // Set maximum load value
    pit_load_value(&PeriodicTimerSource::Timer1, 0xFFFF_FFFF);
    pit_load_value(&PeriodicTimerSource::Timer0, 0xFFFF_FFFF);

    // Secret sauce which makes it all work otherwise you are bound
    // to a default timeout that takes like a minute.
    pit_restart(&PeriodicTimerSource::Timer1);
    pit_restart(&PeriodicTimerSource::Timer0);
}

pub fn nanos() -> u64 {
    let base = pit_read_lifetime();
    return base * 15;
}