//! This module represents paged memory functionality alloc(), and free().
//!
//! Used mostly by internal packages. Be aware that most things which get
//! alloc()'d need to be free()'d so the memory can be relcaimed.
//!
//! Use at your own risk. Most system datastructures that are included
//! with teensycore properly handle memory as best they can, and offer
//! a `drop()` method which should be invoked as soon as the variable
//! is no longer required.
use core::mem::size_of;

#[cfg(not(feature = "testing"))]
use crate::phys::addrs::OCRAM2;

const MEMORY_MAXIMUM: u32 = 0x7_FFFF; // 512kb
const MEMORY_BEGIN_OFFSET: u32 = 0x0_0FFC; // 4kb buffer (note: it should be word aligned)
static mut MEMORY_OFFSET: u32 = MEMORY_BEGIN_OFFSET;
static mut MEMORY_PAGES: Option<*mut Mempage> = None;
static mut IS_OVERRUN: bool = false;

/// A page of memory
#[repr(C)]
pub struct Mempage {
    pub size: usize,
    pub used: bool,
    pub next: Option<*mut Mempage>,
    pub ptr: *mut u32,
}

#[cfg(not(feature = "testing"))]
impl Mempage {
    pub const fn new(size: usize, ptr: *mut u32) -> Self {
        return Mempage {
            size: size,
            used: true,
            ptr: ptr,
            next: None,
        };
    }

    pub fn reclaim(bytes: usize) -> *mut u32 {
        // Iterate through mempage searching for the best candidate
        // that is currently free.
        unsafe {
            let mut best_ptr: Option<*mut Mempage> = None;
            let mut best_size: usize = usize::MAX;

            let mut ptr = MEMORY_PAGES;

            while ptr.is_some() {
                let node = ptr.unwrap();
                if (*node).size >= bytes && best_size > (*node).size && (*node).used == false {
                    best_ptr = Some(node);
                    best_size = (*node).size;
                }
                ptr = (*node).next;
            }

            // Process the best candidate
            match best_ptr {
                None => {}
                Some(node) => {
                    (*node).used = true;
                    return node as *mut u32;
                }
            }
        }

        loop {
            crate::err(crate::PanicType::Memfault);
        }
    }

    pub fn reclaim_fast(bytes: usize) -> *mut u32 {
        // Iterate through mempage searching for the first candidate
        // that is currently free.
        unsafe {
            let mut ptr = MEMORY_PAGES;
            while ptr.is_some() {
                let node = ptr.unwrap();
                if (*node).size >= bytes && (*node).used == false {
                    (*node).used = true;
                    return node as *mut u32;
                }
                ptr = (*node).next;
            }
        }

        loop {
            crate::err(crate::PanicType::Memfault);
        }
    }

    /// Free the page containing this ptr
    pub fn free(ptr: u32) {
        let bytes = size_of::<Mempage>() as u32;
        // We know the Mempage header is
        // right above the pointer. So we can use
        // that knowledge to go straight there.
        let addr = (ptr - bytes) as *mut Mempage;
        unsafe {
            (*addr).used = false;
        }
    }

    pub fn add_page<T>(bytes: usize) -> *mut T {
        let page_bytes = size_of::<Mempage>();
        let mut total_bytes = page_bytes + bytes;

        // Word align
        while total_bytes % 4 != 0 {
            total_bytes += 1;
        }

        let next_page = alloc_bytes(total_bytes) as *mut Mempage;
        let item_ptr = ((next_page as u32) + page_bytes as u32) as *mut T;

        if is_overrun() {
            // Don't allocate a new page
            return item_ptr;
        }

        unsafe {
            (*next_page) = Mempage {
                size: total_bytes,
                ptr: item_ptr as *mut u32,
                used: true,
                next: None,
            };

            match MEMORY_PAGES {
                None => {
                    MEMORY_PAGES = Some(next_page);
                }
                Some(head) => {
                    (*next_page).next = Some(head);
                    MEMORY_PAGES = Some(next_page);
                }
            }
        }

        return item_ptr;
    }
}

/// A debug method which returns true if we've begun
/// recyclilng memory.
pub fn is_overrun() -> bool {
    return unsafe { IS_OVERRUN };
}

/// A method to zero out every piece of memory.
/// If we encounter a bad sector, the device will throw an oob
/// irq and enter error mode.
#[cfg(not(feature = "testing"))]
pub fn memtest() {
    for addr in MEMORY_BEGIN_OFFSET..MEMORY_MAXIMUM / 4 {
        unsafe {
            let ptr = (OCRAM2 + addr * 4) as *mut u32;
            *ptr = 0;
        }
    }
}

/// This method will zero out a certain amount of bytes
/// from a particular address.
#[cfg(not(feature = "testing"))]
pub fn zero(addr: u32, bytes: u32) {
    for byte in 0..bytes {
        unsafe {
            let ptr = (addr + byte) as *mut u8;
            *ptr = 0;
        }
    }
}

/// This method will copy a certain amount of bytes
/// from src and into dest.
#[cfg(not(feature = "testing"))]
pub fn copy(src: u32, dest: u32, len: u32) {
    for byte in 0..len {
        unsafe {
            let src_ptr = (src + byte) as *mut u8;
            let dst_ptr = (dest + byte) as *mut u8;
            *dst_ptr = *src_ptr;
        }
    }
}

/// Internal use only.
///
/// This method will allocate bytes on the heap and
/// return a raw pointer to the freshly claimed area
/// of memory.
#[cfg(not(feature = "testing"))]
fn alloc_bytes(bytes: usize) -> *mut u32 {
    // Check for boundaries and reset if applicable.
    unsafe {
        if MEMORY_OFFSET + bytes as u32 >= MEMORY_MAXIMUM {
            IS_OVERRUN = true;
            return Mempage::reclaim_fast(bytes);
        }

        let ptr = (OCRAM2 + MEMORY_OFFSET) as *mut u32;
        MEMORY_OFFSET += bytes as u32;
        return ptr;
    }
}

/// This method will allocate a page of memory
/// of any discreet struct. It then returns
/// a raw pointer<T> to the location it just
/// established.
#[cfg(not(feature = "testing"))]
pub fn alloc<T>() -> *mut T {
    let bytes = size_of::<T>();
    return Mempage::add_page(bytes);
}

/// Free a pointer by updating the pagefile, allowing
/// other alloc() requests to begin reusing that space.
#[cfg(not(feature = "testing"))]
pub fn free<T>(ptr: *mut T) {
    let zero_ptr = ptr as u32;
    Mempage::free(zero_ptr);
}

#[cfg(feature = "testing")]
pub fn alloc<T>() -> *mut T {
    return unsafe { std::alloc::alloc(std::alloc::Layout::new::<T>()) as *mut T };
}

#[cfg(feature = "testing")]
pub fn free<T>(_ptr: *mut T) {
    // Do nothing
}

#[cfg(feature = "testing")]
pub fn zero(addr: u32, bytes: u32) {}

#[cfg(feature = "testing")]
pub fn copy(src: u32, dest: u32, len: u32) {}
