use crate::fastdivide::*;
/**
 *  This is a system device which keeps track of time by using the periodic timer 
 **/
use crate::phys::*;
use crate::phys::periodic_timers::*;    

static mut HAS_OVERFLOWED: bool = false;
static mut CLOCK_DIVIDER: Option<DividerU64> = None;
static mut OFFSET: u64 = 0;

pub type uNano = u128;

pub fn clock_init() {

    // Setup clock
    periodic_timers::pit_start_clock();
    
    // // Undo clock gating
    assign(addrs::CCM_CCGR1, read_word(addrs::CCM_CCGR1) | (0x3 << 12));
    
    // Setup clock divider used for time keeping.
    unsafe {
        HAS_OVERFLOWED = false;
        CLOCK_DIVIDER = Some(DividerU64::divide_by(1848));
        OFFSET = 0;
    }
    
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


pub fn has_overflowed() -> bool {
    return unsafe { HAS_OVERFLOWED };
}

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
    return match unsafe { CLOCK_DIVIDER } {
        None => 0,
        Some(divider) => {

            let max: u128 = u64::MAX as u128;
            let time: u128 = pit_read_lifetime() * 14000;
            
            // If we have overflowed a u64 then
            // calculate how much we have overflowed, save that
            // reset the timer, and then start again.
            if time > max {
                unsafe { HAS_OVERFLOWED = true };                               
            }

            return ((pit_read_lifetime() * 14000) / 1848) as uNano;
            //return divider.divide((pit_read_lifetime() * 14000) as u64);
        }
    }
}