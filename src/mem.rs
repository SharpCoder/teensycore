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

pub type ScopeUnit = u32;

const MEMORY_MINIMUM: u32 = 0x0_0FFC;
const MEMORY_MAXIMUM: u32 = 0x7_FFFF - 0x0_0FFC; // 512kb - 4kb buffer
const MEMORY_BEGIN_OFFSET: u32 = MEMORY_MINIMUM; // 4kb buffer (note: it should be word aligned)
pub static mut MEMORY_SCOPE: ScopeUnit = 0x1337; // A not-thread-safe reference to the scope in which memory was allocated
static mut MEMORY_OFFSET: u32 = MEMORY_BEGIN_OFFSET;
static mut MEMORY_PAGES: Option<*mut Mempage> = None;
static mut IS_OVERRUN: bool = false;

/// A page of memory
#[repr(C)]
pub struct Mempage {
    pub size: usize,
    pub scope: ScopeUnit,
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
            scope: 0x1337,
            next: None,
        };
    }

    /// Returns how many blocks of memory are actively allocated.
    pub fn ref_count() -> usize {
        let mut count = 0;
        unsafe {
            let mut ptr = MEMORY_PAGES;
            while ptr.is_some() {
                let node = ptr.unwrap();
                if (*node).used == true {
                    count += 1;
                }
                ptr = (*node).next;
            }
        }

        return count;
    }

    /// Returns the next available block of memory that
    /// will fit some arbitrary amount of bytes.
    pub fn reclaim_fast(bytes: usize) -> *mut u32 {
        // Iterate through mempage searching for the first candidate
        // that is currently free.
        unsafe {
            let mut ptr = MEMORY_PAGES;

            while ptr.is_some() {
                let node = ptr.unwrap();
                if (*node).size >= bytes && (*node).used == false {
                    (*node).used = true;
                    (*node).scope = MEMORY_SCOPE;
                    return node as *mut u32;
                }
                ptr = (*node).next;
            }
        }

        loop {
            crate::err(crate::PanicType::Memfault);
        }
    }

    /// Release all memory that was allocated with a given scope.
    pub fn free_scope(scope: ScopeUnit) {
        // Iterate through mempage dropping all memory allocated with a given scope
        unsafe {
            let mut ptr = MEMORY_PAGES;
            while ptr.is_some() {
                let node = ptr.unwrap();
                if (*node).scope == scope && (*node).used == true {
                    Mempage::free((*node).ptr as u32);
                }
                ptr = (*node).next;
            }
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
                scope: MEMORY_SCOPE,
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
            // return Mempage::reclaim(bytes);
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

#[cfg(not(feature = "testing"))]
#[macro_export]

/// A directive for managing memory.
///
/// Any memory that is dynamically allocated within the critical block
/// of this macro will be released at the end. This is extremely useful
/// for string-based operations which may have many side effects. You
/// don't have to fuss with drop().
///
/// ```no-test
/// use teensycore::*;
/// use teensycore::mem::*;
///
/// using!({
///     let var1 = str!(b"hello!");
///     let var2 = str!(b"world!");
/// });
/// ```
macro_rules! using {
    ($x: block) => {
        {
            // Record the original scope that memory is currently being allocated against
            // and then establish a new scope based on the line of code currently
            // executing. With this scope, all subsequent memory will be allocated against.
            // After executing the critical block, release all memory allocated recently
            // and return the scope to the original.
            let original_scope: ScopeUnit = unsafe { MEMORY_SCOPE.clone() };
            let current_scope: ScopeUnit = crate::code_hash();
            unsafe { MEMORY_SCOPE = current_scope };

            $x

            // Deallocate all memory in the current_scope
            Mempage::free_scope(current_scope);
            unsafe { MEMORY_SCOPE = original_scope; }
        }
    }
}

pub fn ref_count() -> usize {
    return Mempage::ref_count();
}

#[cfg(feature = "testing")]
#[macro_export]
macro_rules! using {
    ($x: block) => {{
        $x
    }};
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
