#![feature(lang_items, fn_traits)]
#![crate_type = "staticlib"]
#![no_std]
pub mod clock;
pub mod debug;
pub mod gate;
pub mod math;
pub mod mem;
pub mod phys;
pub mod serio;
pub mod system;

use core::arch::asm;
use core::arch::global_asm;    
use phys::*;
use phys::irq::*;
use phys::pins::*;
use system::map::*;
use system::strings::*;
use system::vector::*;

pub const S_TO_NANO: u64 = 1000000000;
pub const MS_TO_NANO: u64 = S_TO_NANO / 1000;   

pub static mut GATES: BTreeMap::<u32, u32> = BTreeMap {
    root: None,
};

#[derive(Copy, Clone)]
pub struct Box<T : Clone + Copy> {
    reference: T,
}

impl <T : Clone + Copy> Box<T> {
    pub fn new(reference: T) -> Self {
        return Box { reference: reference };
    }
}

#[macro_export]
macro_rules! main {
    ($app_code: block) => {
        use teensycore::*;
        use teensycore::clock::*;
        use teensycore::phys::*;
        use teensycore::phys::irq::*;
        use teensycore::serio::*;

        #[no_mangle]
        fn main() {
            // Initialize irq system, (disables all interrupts)
            disable_interrupts();

            // Initialize clocks
            phys_clocks_en();

            // Ignite system clock for keeping track of millis()
            // which is also used for the wait implementation.
            clock_init();

            // Setup serial
            serial_init(SerioDevice::Default);
            serial_init(SerioDevice::Debug);

            // Enable interrupts across the system
            enable_interrupts();
            
            $app_code
        }
        
        #[lang = "eh_personality"]
        #[no_mangle]
        pub extern fn eh_personality() {}#[panic_handler]

        #[no_mangle]
        pub extern fn my_panic(_info: &core::panic::PanicInfo) -> ! {
            loop { }
        }
    }
}


#[derive(Copy, Clone)]
pub struct SystemMessage {
    pub topic: String,
    pub content: String,
}

pub trait Task {
    fn new() -> Self;
    fn init(&mut self);
    fn system_loop(&mut self);
}

pub fn wait_ns(nano: u64) {
    unsafe {
        let origin = clock::nanos();
        let target = nano;
        while (origin + target) > clock::nanos() {
            asm!("nop");
        }
    }
}

pub fn pendsv() {
    unsafe {
        *((0xE000ED04) as *mut u32) = 0x10000000;
    }
}

pub fn dsb() {
    unsafe {
        asm!("dsb");
    }
}

pub enum PanicType {
    Hardfault,
    Memfault,
    Oob,
}

#[no_mangle]
pub fn err(mode: PanicType) {
    disable_interrupts();
    loop {
        match mode {
            PanicType::Hardfault => {
                pin_out(13, Power::High);
            },
            PanicType::Oob => {
                pin_out(13, Power::High);
                wait_ns(MS_TO_NANO * 50);
                pin_out(13, Power::Low);
                wait_ns(MS_TO_NANO * 500);
            },
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
pub fn oob() {
    err(PanicType::Oob);
}

/// This function returns a u32 containing the
/// program counter of the line of code which
/// invokes this function.
pub fn code_hash() -> u32 {
    let result = 0;
    unsafe {
        asm!("mov r0, lr");
    }
    return result;
}

global_asm!("
    _ZN4core9panicking18panic_bounds_check17h9048f255eeb8dcc3E:
        bl oob
        b hang

    hang:
        b hang
");