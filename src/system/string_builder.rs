//! StringBuilder is a module with functionality relating to
//! string operations. It is an efficient datastructure that
//! leverages heap-allocation, but re-uses memory whenever
//! possible.
//! 
//! StringBuilder is implemented with a block-allocation
//! strategy, where segments of memory are reserved as
//! chunks of bytes (or pages). This allows reduced calls 
//! to alloc, as well as efficient array operations 
//! that are optimized for insert and lookup. 
//! 
//! This implementation does not lend itself well to
//! removing individual items at arbitrary indexes.
//! For now, such functionality is simply not implemented.
//! If you need stack or queue like operations, consider
//! a Vector instead.

use crate::{mem::*, math::min};

const CHAR_BLOCK_SIZE: usize = 32;

/// A CharBlockNode is a block of characters allocated
/// at one time. This allows us to be a bit more
/// forward-thinking with the burden of memory
/// allocation.
struct CharBlockNode {
    data: [u8; CHAR_BLOCK_SIZE],
    used: usize,
    next: Option<*mut CharBlockNode>,
}

pub struct StringBuilder {
    head: Option<*mut CharBlockNode>,
    tail: Option<*mut CharBlockNode>,
    capacity: Option<usize>,
    index: usize,
    blocks: usize,
}

impl StringBuilder {

    pub fn new() -> Self {
        return StringBuilder {
            blocks: 0,
            capacity: None,
            head: None,
            tail: None,
            index: 0,
        };
    }

    /// Create a new instance of a string builder, capped at a maximum
    /// capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut result = StringBuilder {
            blocks: 0,
            capacity: Some(capacity),
            head: None,
            tail: None,
            index: 0,
        };

        return result;
    }

    /// Return the length of bytes used inside the buffer.
    pub fn len(&self) -> usize {
        return self.index;
    }

    pub fn clear(&mut self) {
        // Iterate through each block and reset "used".
        let mut ptr = self.head;
        loop {
            if ptr.is_none() {
                break;
            }

            let node = ptr.unwrap();
            unsafe {
                (*node).used = 0;
                ptr = (*node).next;
            }
        }

        self.index = 0;
        self.tail = self.head;
    }

    /// Returns a new StringBuilder containing the characters
    /// between the indexes.
    pub fn slice(&self, start: usize, end: usize) -> StringBuilder {
        if start > end || end > self.index || self.head.is_none() {
            return StringBuilder::new();
        }

        let mut blocks = 0;
        let mut idx = 0;
        let mut ptr = self.head;
        let mut slice = StringBuilder::new();

        loop {
            if ptr.is_none() {
                break;
            }

            let node = ptr.unwrap();
            if start > idx + CHAR_BLOCK_SIZE {
                // We can skip this module
            } else if start > idx {
                // We need to process some of this page,
                // but it starts kinda in the middle.
                let offset = start - blocks * CHAR_BLOCK_SIZE;
                let bytes_to_copy = min(end - start + 1, CHAR_BLOCK_SIZE);

                for i in 0 ..= bytes_to_copy {
                    slice.append(&[unsafe { (*node).data[offset + i] }]);
                }
                
            } else {
                // We need to process some or all of this page
                // and it starts from the beginning, so we can
                // use bulk copy method
                let bytes_to_copy = min(end - idx - 1, CHAR_BLOCK_SIZE - 1);
                slice._copy(unsafe { &(*node).data }, bytes_to_copy);
            }

            blocks += 1;
            idx += CHAR_BLOCK_SIZE;
            ptr = unsafe { (*node).next };

            if idx > end {
                break;
            }
        }

        return slice;
    }

    /// Return the character inside the buffer at a given position.
    pub fn char_at(&self, index: usize) -> Option<u8> {
        if index > self.index {
            return None;
        }

        let block = index / CHAR_BLOCK_SIZE;
        let mut ptr = self.head.unwrap();

        for _ in 0 .. block {
            ptr = unsafe { (*ptr).next.unwrap() };
        }


        let access_point = index - (block * CHAR_BLOCK_SIZE);
        return Some(unsafe { (*ptr).data[access_point] });
    }

    /// Append a static array of ascii characters to the buffer.
    /// If this operation would result in a buffer overflow,
    /// the append is aborted and the function will return false
    /// to indicate that data has been lost.
    pub fn append(&mut self, chars: &[u8]) -> bool {
        return self._copy(chars, chars.len());
    }

    /// Add all characters from another StringBuilder into self.
    pub fn join(&mut self, other: &StringBuilder) -> bool {
        // If the other string is empty, we can abort.
        if other.head.is_none() {
            return true;
        }

        // Copy each block
        let mut ptr = other.head;
        let mut ret = true;

        loop {
            if ptr.is_none() {
                break;
            }

            let node = ptr.unwrap();
            let block = unsafe { (*node).data };
            ret = self._copy(&block, unsafe { (*node).used });
            ptr = unsafe { (*node).next };
        }

        return ret;
    }

    /// Internal function to copy a certain amount of bytes
    /// from an array into self.
    fn _copy(&mut self, data: &[u8], len: usize) -> bool {
        if self.head.is_none() {
            self._allocate_block();
        }

        let bytes_to_copy = min(len, data.len());

        // Verify we do not over-extend capacity.
        match self.capacity {
            None => { },
            Some(capacity) => {
                if self.index + bytes_to_copy > capacity {
                    self._buffer_overflow();
                    return false;
                }
            }
        }
        
        let mut tail = self.tail.unwrap();
        for i in 0 .. bytes_to_copy {
            if unsafe { (*tail).used == CHAR_BLOCK_SIZE } {
                self._allocate_block();
                tail = self.tail.unwrap();
            }
    
            // Place the character in the spot
            unsafe {
                let block_index = (*tail).used;
                (*tail).data[block_index] = data[i];
                (*tail).used += 1;
            }
            self.index += 1;
        }

        return true;
    }

    /// This method is invoked when a buffer overflow happens.
    fn _buffer_overflow(&self) {

    }

    /// Allocates a new block at the end
    /// of the buffer, if necessary.
    /// 
    /// This method is aware of orphaned blocks
    /// and will re-use them as-needed.
    fn _allocate_block(&mut self) {

        // Check if we have any orphaned blocks to use.
        if self.tail.is_some() && unsafe { (*self.tail.unwrap()).next.is_some() } {
            // Update tail
            self.tail = unsafe { (*self.tail.unwrap()).next };
            return;
        }


        let block = alloc();
        self.blocks += 1;

        unsafe { 
            (*block) = CharBlockNode {
                data: [0; CHAR_BLOCK_SIZE],
                next: None,
                used: 0,
            };
        }

        match self.tail {
            None => {
                // Add to head
                self.head = Some(block);
                self.tail = self.head;
            },
            Some(node) => {
                // Add to the node
                unsafe { (*node).next = Some(block) };
                self.tail = Some(block);
            }
        }
    }
}

#[cfg(test)]
mod test_string_builder {

    use super::*;

    fn sb_equals(sb: StringBuilder, text: &[u8]) {
        for i in 0 .. text.len() {
            assert_eq!(sb.char_at(i).unwrap(), text[i]);
        }
    }

    #[test]
    fn test_string_builder() {
        let mut sb = StringBuilder::new();
        sb.append(b"hello, world");
        sb.append(b"this has many characters in it. more than 32");

        assert_eq!(sb.blocks, 2);
        assert_eq!(sb.char_at(0), Some(b'h'));
        assert_eq!(sb.char_at(5), Some(b','));
    }

    #[test]
    fn test_capacity() {
        let mut sb = StringBuilder::with_capacity(4);
        assert_eq!(sb.append(b"more than 4"), false);
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn test_join() {
        let mut sb = StringBuilder::new();
        let mut sb2 = StringBuilder::new();

        sb.append(b"hello, ");
        sb2.append(b"world");

        // Join them
        sb.join(&sb2);

        assert_eq!(sb.len(), 12);
    }

    #[test]
    fn test_clear() {
        let mut sb = StringBuilder::new();
        sb.append(b"hello, world");

        // Clear string builder and then add more stuff to it
        // and see if it allocates a new block.
        sb.clear();
        assert_eq!(sb.len(), 0);
        assert_eq!(sb.blocks, 1);
        sb.append(b"g'morning world");
        assert_eq!(sb.len(), 15);
        assert_eq!(sb.blocks, 1);
    }

    #[test]
    fn test_slice() {
        let mut sb = StringBuilder::new();
        sb.append(b"g'morning ");
        sb.append(b"world");

        let slice = sb.slice(2, 7);
        sb_equals(slice, b"morning");

        // Test an extremely large slice, more than a whole page
        sb.append(b"this is more than a whole page of data");
        let slice2 = sb.slice(10, 48);

    }
}