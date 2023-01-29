//! Irq module deals with interrupt handling.
//!
//! In this module you will find functions that help
//! manaage interrupts. Here is an example of enabling
//! interrupts for the periodic timer:
//!
//! ```no_run
//! use teensycore::phys::irq::*;
//!
//! irq_attach(Irq::PeriodicTimer, handle_timer_irq);
//! irq_enable(Irq::PeriodicTimer);
//!
//! fn handle_timer_irq() {
//!
//! }
//! ```
#![allow(dead_code)]

type Fn = fn();
use crate::{
    assembly,
    phys::{addrs, assign, assign_8, read_word, set_bit},
};
use core::arch::asm;

// On the teensy, it's actually closer to 158 interrupts
// This will be adjusted in the future.
const MAX_SUPPORTED_IRQ: usize = 256;

#[repr(C)]
pub struct IrqTable {
    pub init_sp: u32,
    pub reset_handler: Fn,
    pub nmi_handler: Fn,
    pub hardfault_handler: Fn,
    pub mpufault_handler: Fn,
    pub busfault_handler: Fn,
    pub usagefault_handler: Fn,
    pub rsv0: u32,
    pub rsv1: u32,
    pub rsv2: u32,
    pub rsv3: u32,
    pub svc_handler: Fn,
    pub rsv4: u32,
    pub rsv5: u32,
    pub pendsv_handler: Fn,
    pub systick_handler: Fn,
    pub interrupts: [Fn; MAX_SUPPORTED_IRQ],
}

pub static mut VECTORS: IrqTable = IrqTable {
    init_sp: 0x00, // This should probably not be 0.
    reset_handler: noop,
    nmi_handler: fault_handler,
    hardfault_handler: fault_handler,
    mpufault_handler: fault_handler,
    busfault_handler: fault_handler,
    usagefault_handler: fault_handler,
    rsv0: 0x0,
    rsv1: 0x0,
    rsv2: 0x0,
    rsv3: 0x0,
    svc_handler: noop,
    rsv4: 0x0,
    rsv5: 0x0,
    pendsv_handler: noop,
    systick_handler: noop,
    interrupts: [noop; MAX_SUPPORTED_IRQ],
};

/** Interrupts */
#[derive(Copy, Clone)]
pub enum Irq {
    Uart1 = 20,
    Uart2 = 21,
    Uart3 = 22,
    Uart4 = 23,
    Uart5 = 24,
    Uart6 = 25,
    Uart7 = 26,
    Uart8 = 29,
    UsbPhy1 = 65, // UTMI0
    UsbPhy2 = 66, // UTMI1
    Gpt1 = 100,
    Gpt2 = 101,
    Usb1 = 113, // USB OTG1
    Usb2 = 112, // USB OTG2
    PeriodicTimer = 122,
}

static mut IRQ_DISABLE_COUNT: usize = 0;

/// System-level command to resume processing interrupts
/// across the device.
///
pub fn enable_interrupts() {
    unsafe {
        if IRQ_DISABLE_COUNT > 0 {
            IRQ_DISABLE_COUNT -= 1;
        }
    }

    if unsafe { IRQ_DISABLE_COUNT } == 0 {
        assembly!("CPSIE i");
    }
}

/// System-level command to stop processing interrupts
/// across the device.
///
pub fn disable_interrupts() {
    unsafe {
        IRQ_DISABLE_COUNT += 1;
    }

    assembly!("CPSID i");
}

/// Return the current address stored
/// in the NVIC
fn irq_addr() -> u32 {
    return read_word(0xe000ed08);
}

/// Return the total size of the IVT
fn irq_size() -> u32 {
    return core::mem::size_of::<IrqTable>() as u32;
}

/// Get the current IVT wherever it may be stored
fn get_ivt() -> *mut IrqTable {
    return irq_addr() as *mut IrqTable;
}

/// Enable a specific interrupt
pub fn irq_enable(irq_number: Irq) {
    let num = irq_number as u32;
    let bank = num / 32;
    let bit = num - bank * 32;
    let addr = addrs::NVIC_IRQ_ENABLE_REG + (bank * 4);
    let original_value = read_word(addr);
    let next_value = set_bit(original_value, bit as u8);
    assign(addr, next_value);
}

/// Disable a specific interrupt
pub fn irq_disable(irq_number: Irq) {
    let num = irq_number as u32;
    let bank = num / 32;
    let bit = num - bank * 32;
    let addr = addrs::NVIC_IRQ_CLEAR_REG + (bank * 4);
    let original_value = read_word(addr);
    let next_value = set_bit(original_value, bit as u8);
    assign(addr, next_value);
}

/// Set a particular Irq with a given priority.
///
/// The lower the priority, the more important the interrupt will be.
pub fn irq_priority(irq_number: Irq, priority: u8) {
    let num = irq_number as u32;
    put_irq_priority(num, priority);
}

pub fn irq_clear_pending() {
    assign(addrs::NVIC_IRQ_CLEAR_PENDING_REG + 0x0, 0x0);
    assign(addrs::NVIC_IRQ_CLEAR_PENDING_REG + 0x4, 0x0);
    assign(addrs::NVIC_IRQ_CLEAR_PENDING_REG + 0x8, 0x0);
    assign(addrs::NVIC_IRQ_CLEAR_PENDING_REG + 0xC, 0x0);
}

/**
This method exists to copy the "shadow NVIC" into
the real NVIC.

Why?

The actual address stored in the NVIC changes sometimes.
I don't know why. But it seems like the data gets copied
to a new location randomly. I have my theories, and it
seems to only happen after the stack pointer
is moved around. So anyway, this is just the nuclear
approach to really fkn thoroughly making sure
the NVIC has the value I think it has.

Spent like 20 hours debugging this. I am so done
with magic memory locations changing around.
*/
fn update_ivt() {
    let ivt = get_ivt();
    unsafe {
        // We have no idea here
        // (*ivt).init_sp =  VECTORS.init_sp;
        (*ivt).reset_handler = VECTORS.reset_handler;
        (*ivt).nmi_handler = VECTORS.nmi_handler;
        (*ivt).hardfault_handler = VECTORS.hardfault_handler;
        (*ivt).mpufault_handler = VECTORS.mpufault_handler;
        (*ivt).busfault_handler = VECTORS.busfault_handler;
        (*ivt).usagefault_handler = VECTORS.usagefault_handler;
        (*ivt).rsv0 = VECTORS.rsv0;
        (*ivt).rsv1 = VECTORS.rsv1;
        (*ivt).rsv2 = VECTORS.rsv2;
        (*ivt).rsv3 = VECTORS.rsv3;
        (*ivt).svc_handler = VECTORS.svc_handler;
        (*ivt).rsv4 = VECTORS.rsv4;
        (*ivt).rsv5 = VECTORS.rsv5;
        (*ivt).svc_handler = VECTORS.svc_handler;
        (*ivt).pendsv_handler = VECTORS.pendsv_handler;
        (*ivt).systick_handler = VECTORS.systick_handler;
        let mut i = 0;
        while i < MAX_SUPPORTED_IRQ {
            (*ivt).interrupts[i] = VECTORS.interrupts[i];
            i += 1;
        }
    }
}

// Internal method for assigning a specific irq
// at a specific index to the IVT.
fn put_irq(irq_number: usize, ptr: Fn) {
    unsafe {
        // Update shadow copy
        VECTORS.interrupts[irq_number] = ptr;
    }
    // Copy shadow to actual NVIC
    update_ivt();
}

/// Set a particular IRQ with a given priority.
/// The lower the priority value, the more important
/// the interrupt will be.
fn put_irq_priority(irq_number: u32, priority: u8) {
    let addr = addrs::NVIC_IRQ_PRIORITY_REG + irq_number;
    assign_8(addr, priority);
}

// DO NOT USE!!!
// Unless you know what you are doing
pub fn fill_irq(func: Fn) {
    let mut index = 0;
    while index < MAX_SUPPORTED_IRQ {
        unsafe {
            VECTORS.interrupts[index] = func;
        }
        index += 1;
    }
    update_ivt();
}

/// Public method for attaching an interrupt to an
/// enum-gated IRQ source.
pub fn irq_attach(irq_number: Irq, func: Fn) {
    put_irq(irq_number as usize, func);
}

/// Some kind of hard-fault, typically
/// this is a catastrophic function that hangs
/// the program.
fn fault_handler() {
    crate::err(crate::PanicType::Hardfault);
}

/// An un-implemented interrupt
fn noop() {
    assembly!("nop");
}
