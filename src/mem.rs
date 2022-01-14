use core::mem::{size_of};

#[cfg(not(test))]
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

#[cfg(not(test))]
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
                None => {},
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
                },
                Some(head) => {
                    (*next_page).next = Some(head);
                    MEMORY_PAGES = Some(next_page);
                }
            }
        }

        return item_ptr;

    }
}

pub fn is_overrun() -> bool {
    return unsafe { IS_OVERRUN };  
}

/// zero out every piece of memory.
/// if we encounter a bad sector,
/// the device will throw an oob irq
/// and enter error mode.
#[cfg(not(test))]
pub fn memtest() {
    for addr in MEMORY_BEGIN_OFFSET .. MEMORY_MAXIMUM / 4 {
        unsafe {
            let ptr = (OCRAM2 + addr * 4) as *mut u32;
            *ptr = 0;
        }
    }
}

#[cfg(not(test))]
pub fn alloc_bytes(bytes: usize) -> *mut u32 {
    // Check for boundaries and reset if applicable.
    unsafe {
        if MEMORY_OFFSET + bytes as u32 >= MEMORY_MAXIMUM {
            IS_OVERRUN = true;
            return Mempage::reclaim(bytes);
        }

        let ptr = (OCRAM2 + MEMORY_OFFSET) as *mut u32;
        MEMORY_OFFSET += bytes as u32;
        return ptr;
    }
}

#[cfg(not(test))]
pub fn alloc<T>() -> *mut T {
    let bytes = size_of::<T>();
    return Mempage::add_page(bytes);
}

/// Free a pointer by updating the pagefile
#[cfg(not(test))]
pub fn free<T>(ptr: *mut T) {
   let zero_ptr = ptr as u32;
    Mempage::free(zero_ptr);
}

#[cfg(test)]
pub fn alloc<T>() -> *mut T {
    return unsafe { std::alloc::alloc(std::alloc::Layout::new::<T>()) as *mut T };
}

#[cfg(test)]
pub fn free<T>(_ptr: *mut T) {
    // Do nothing
}
