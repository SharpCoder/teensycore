//! A kernel for teensy-4.0 microcontroller.
//!
//! This crate provides all the resources necessary to
//! begin interfacing with the teensy-4.0. To get started,
//! you simply need to use the main! macro. This will perform
//! a number of chores for you including the following:
//!
//!  - Initialize uart, xbar, gpio, system clock
//!  - Enable interrupts
//!  - Add panic handling
//!  - Verify memory access
//!  - Enable FPU

#![no_std]
#![allow(internal_features)]
#![feature(lang_items, fn_traits)]
#![crate_type = "staticlib"]

#[cfg(feature = "testing")]
extern crate std;

pub mod clock;
pub mod debug;
pub mod gate;
pub mod i2c;
pub mod math;
pub mod mem;
pub mod phys;
pub mod prelude;
pub mod serio;
pub mod system;
pub mod usb_serial;

use crate::clock::uNano;
use core::arch::asm;
use core::arch::global_asm;
use phys::irq::*;
use phys::pins::*;

/// Returns how many nanos are in a second
///
/// ```no_run
/// use teensycore::{*, clock::*};
/// let target_time = nanos() + 2 * S_TO_NANO;
/// loop {
///     if nanos() > target_time {
///         break;
///     }
/// }
/// ```
pub const S_TO_NANO: uNano = 1000000000;

/// Returns how many nanos are in a millisecond
///
/// ```no_run
/// use teensycore::{*, clock::*};
/// let target_time = nanos() + 2 * MS_TO_NANO;
/// loop {
///     if nanos() > target_time {
///         break;
///     }
/// }
/// ```
pub const MS_TO_NANO: uNano = S_TO_NANO / 1000;

/// Returns how many nanos are in a microsecond
///
/// ```no_run
/// use teensycore::{*, clock::*};
/// let target_time = nanos() + 2 * MICRO_TO_NANO;
/// loop {
///     if nanos() > target_time {
///         break;
///     }
/// }
/// ```
pub const MICRO_TO_NANO: uNano = 1000;

/// This is the primary macro necessary to bootstrap your application.
/// It takes a code block that will be used as the entrypoint to your
/// logic.
#[macro_export]
macro_rules! main {
    ($app_code: block) => {
        use teensycore::prelude::*;

        pub static mut GATES: BTreeMap<u32, u32> = BTreeMap { root: None };

        #[no_mangle]
        pub fn main() {
            loop {
                // Initialize irq system, (disables all interrupts)
                disable_interrupts();

                // Initialize clocks
                phys_clocks_en();

                // Ignite system clock for keeping track of millis()
                clock_init();

                // Make the LED pin an output
                pin_mode(13, Mode::Output);

                // Setup serial
                serial_init(SerioDevice::Default);

                // Enable interrupts across the system
                enable_interrupts();

                usb_initialize();
                usb_serial_init();

                $app_code
            }
        }

        #[lang = "eh_personality"]
        #[no_mangle]
        pub extern "C" fn eh_personality() {}
        #[panic_handler]
        #[no_mangle]
        pub extern "C" fn my_panic(_info: &core::panic::PanicInfo) -> ! {
            loop {}
        }
    };
}

pub trait Task {
    fn new() -> Self;
    fn init(&mut self);
    fn system_loop(&mut self);
}

#[cfg(not(feature = "testing"))]
#[macro_export]
/// This method will call the asm! macro but
/// in a way that doesn't break tests. Use it
/// in lieu of the `asm!` macro.
macro_rules! assembly {
    ($asm: tt) => {
        unsafe {
            asm!($asm);
        }
    };
}

#[cfg(feature = "testing")]
#[macro_export]
macro_rules! assembly {
    ($asm: tt) => {};
}

/// Waits for a specific amount of nanoseconds.
///
/// You can compose this with `S_TO_NANOS` or `MS_TO_NANOS`
/// for easier control over the time.
///
/// ```no_run
/// use teensycore::*;
/// wait_ns(S_TO_NANO * 1);
/// ```
pub fn wait_ns(nano: uNano) {
    wait_exact_ns(nano);
}

#[no_mangle]
fn div(a: f32, b: f32) -> u32 {
    return (a / b) as u32;
}

#[inline]
#[no_mangle]
/// This method will wait a certain amount of milliseconds
/// by calculating how many `nop` commands it will take
/// and issuing them sequentially.
///
/// This is extremely performant and has been used
/// successfully to drive WS2812b LEDs which require
/// nanosecond-specific timing.
pub fn wait_exact_ns(nano: uNano) {
    let cycles = div(nano as f32 - 98.0, 7.54);
    for _ in 0..cycles {
        assembly!("nop");
    }
}

/// This method will intiate a pendsv interrupt
pub fn pendsv() {
    unsafe {
        *((0xE000ED04) as *mut u32) = 0x10000000;
    }
}

/// Data Memory Barrier
pub fn dsb() {
    assembly!("dsb");
}

/// Instruction Synchronization Barrier
pub fn isb() {
    assembly!("isb");
}

// Delete data from the cache, without touching memory
//
// Normally arm_dcache_delete() is used before receiving data via
// DMA or from bus-master peripherals which write to memory.  You
// want to delete anything the cache may have stored, so your next
// read is certain to access the physical memory.
#[no_mangle]
pub fn arm_dcache_delete(addr: u32, size: u32) {
    let mut location = addr & 0xFFFFFFE0;
    let end_addr = addr + size;

    dsb();
    loop {
        phys::assign(0xE000EF5C, location);
        location += 32;

        if location >= end_addr {
            break;
        }
    }

    dsb();
    isb();
}

pub enum PanicType {
    Hardfault,
    Memfault,
    Oob,
}

#[no_mangle]
/// Use this method to enter a system-wide failure event.
///
/// Based on the panic mode, the onboard LED will blink
/// different patterns.
///
/// Hard failure of any generic kind (hardfault)
/// LED is just on forever.
///
/// Out of bounds (oob)
/// LED is on very briefly (50ms) and pulled low for 1.5s.
///
/// Memory Fault (memfault)
/// LED is on for a long time (1.5s) and pulled low briefly (50ms).
///
/// This blink pattern will loop indefinitely and the system will
/// be entirely inoperable. Reserved for catastrophic, non-recoverable
/// situations.
pub fn err(mode: PanicType) {
    disable_interrupts();
    loop {
        match mode {
            PanicType::Hardfault => {
                pin_out(13, Power::High);
            }
            PanicType::Oob => {
                pin_out(13, Power::High);
                wait_ns(MS_TO_NANO * 50);
                pin_out(13, Power::Low);
                wait_ns(MS_TO_NANO * 1500);
            }
            PanicType::Memfault => {
                pin_out(13, Power::High);
                wait_ns(MS_TO_NANO * 1500);
                pin_out(13, Power::Low);
                wait_ns(MS_TO_NANO * 50);
            }
        }
    }
}

#[no_mangle]
/// Issue an out-of-bounds kernel panic.
fn oob() {
    err(PanicType::Oob);
}

/// This function returns a u32 containing the
/// program counter of the line of code which
/// invokes this function.
///
pub fn code_hash() -> u32 {
    let mut result = 0;

    #[cfg(not(feature = "testing"))]
    unsafe {
        asm!(
            "mov {result}, lr",
            result = inout(reg) result
        );
    }

    return result;
}

#[cfg(not(feature = "testing"))]
global_asm!(
    "
    _ZN4core9panicking18panic_bounds_check17h9048f255eeb8dcc3E:
        bl oob
        b hang

    hang:
        b hang
"
);
