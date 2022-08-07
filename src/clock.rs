/**
 *  This is a system device which keeps track of time by using the periodic timer 
 **/
use crate::phys::*;
use crate::phys::periodic_timers::*;    

#[allow(non_camel_case_types)]
pub type uNano = u128;

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

#[no_mangle]
/// This method returns the current uptime of the system
/// in nanoseconds.
pub fn nanos() -> uNano {
    // The periodic timer clock is configured to be 132MHz which
    // is 7.5757575 nanoseconds per tick.
    //
    // In order to achieve perfect timing, we need some division here.
    // Because of how large the number is, we cannot use floats
    // otherwise we'll lose precisino and the value will just be wrong.
    //
    // Through the embarassing process of trial and error, I determined
    // that 14000 / 1848 = 7.575757575. But there's a catch. Doing
    // large-number division after a mnultiplication operation is
    // inherently unstable, because the value could overflow.
    // This is why we are using a third-party division library
    // to do software-level division instead of relying on assembly.
    // 
    // The end result is a perfectly accurate clock, as verified through
    // an external source (a separate arduino).
    let uptime_ticks = pit_read_lifetime() as uNano;
    return ((uptime_ticks * 14000) / 1848) as uNano;
}