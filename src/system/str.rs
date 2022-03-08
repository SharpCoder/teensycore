//! Str is a module with functionality relating to
//! string operations. It is an efficient datastructure that
//! leverages heap-allocation, but re-uses memory whenever
//! possible.
//! 
//! Str is implemented with a block-allocation
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
use crate::system::vector::*;
use core::{iter::{Iterator, IntoIterator}, cmp::Ordering};

const CHAR_BLOCK_SIZE: usize = 32;

/// A thin wrapper around Str::with_content($X)
/// 
/// Use this to create an Str object without having
/// to namespace anything.
/// 
/// ```no-test
/// use teensycore::*;
/// use teensycore::system::str::*;
/// 
/// let var = str!(b"hello, world!");
/// ```
#[macro_export]
macro_rules! str {
    ( $x: expr ) => {{
        Str::with_content($x)
    }};
}


pub trait StringOps<T> {
    fn split(&self, target: u8) -> Vector<Str>; 
    fn index_of(&self, target: &T) -> Option<usize>;
    fn contains(&self, target: &T) -> bool;
    fn reverse(&mut self) -> &mut Self;
}

/// A CharBlockNode is a block of characters allocated
/// at one time. This allows us to be a bit more
/// forward-thinking with the burden of memory
/// allocation.
struct CharBlockNode {
    data: [u8; CHAR_BLOCK_SIZE],
    used: usize,
    next: Option<*mut CharBlockNode>,
}

pub struct StrIter {
    current: Option<*mut CharBlockNode>,
    index: usize,
    size: usize,
}

#[derive(Copy, Clone)]
pub struct Str {
    head: Option<*mut CharBlockNode>,
    tail: Option<*mut CharBlockNode>,
    capacity: Option<usize>,
    index: usize,
    blocks: usize,
}

// This device is only 1 thread so... everyone gets a sync!
unsafe impl Sync for Str  {

}

impl Iterator for StrIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => {
                return None;
            },
            Some(node) => {
                let result = unsafe { (*node).data[self.index] };
                self.index += 1;

                if self.index >= CHAR_BLOCK_SIZE {
                    self.index = 0;
                    self.current = unsafe { (*node).next };
                } else if self.index >= self.size {
                    self.index = 0;
                    self.current = None;
                }

                return Some(result);
            }
        }
    }
}


impl Str {

    pub const fn new() -> Self {
        return Str {
            blocks: 0,
            capacity: None,
            head: None,
            tail: None,
            index: 0,
        };
    }

    /// Create a new string from another string.
    /// This operation performs a bulk copy of
    /// byte arrays and returns a new string
    /// with its own lifetime.
    pub fn from_str(other: &Str) -> Self {
        let mut result = Str::new();
        result.join(other);
        return result;
    }

    pub fn with_content(content: &[u8]) -> Self {
        let mut result = Str::new();
        result.append(content);
        return result;
    }

    /// Create a new instance of a string builder, capped at a maximum
    /// capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let result = Str {
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

    /// Returns a new Str containing the characters
    /// between the indexes.
    /// 
    /// This method returns a copy of the data in question,
    /// not a mutable reference. Don't forget to call `drop()`
    /// on the resulting Str instance once you are 
    /// done with it.
    pub fn slice(&self, start: usize, end: usize) -> Str {
        if start > end || end > self.index || self.head.is_none() {
            return Str::new();
        }

        let mut slice = Str::new();

        // TODO: This is extremely inefficient. Improve
        // the efficiency by iterating over blocks
        // and bulk copying them as needed.
        for idx in start ..= end {
            slice.append(&[self.char_at(idx).unwrap()]);
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

    pub fn put(&mut self, index: usize, char: u8) {
        if index > self.index {
            return;
        }

        let block = index / CHAR_BLOCK_SIZE;
        let mut ptr = self.head.unwrap();

        for _ in 0 .. block {
            ptr = unsafe { (*ptr).next.unwrap() };
        }

        let access_point = index - (block * CHAR_BLOCK_SIZE);
        unsafe {
            (*ptr).data[access_point] = char;
        }
    }

    /// Append a static array of ascii characters to the buffer.
    /// If this operation would result in a buffer overflow,
    /// the append is aborted and the function will return false
    /// to indicate that data has been lost.
    pub fn append(&mut self, chars: &[u8]) -> bool {
        return self._copy(chars, chars.len());
    }

    /// Add all characters from another Str into self.
    pub fn join(&mut self, other: &Str) -> bool {
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

    pub fn join_with_drop(&mut self, other: &mut Str) -> bool {
        let ret = self.join(other);
        other.drop();
        return ret;
    }

    /// Returns an iterator of this Str instance.
    pub fn into_iter(&self) -> StrIter {
        return StrIter {
            current: self.head,
            index: 0,
            size: self.index,
        };
    }

    /// This method will deallocate all heap memory
    /// data blocks, rendering this instance of
    /// Str effectively unusable.
    pub fn drop(&mut self) {
        match self.head {
            None => {
                // There is nothing to deallocate
            },
            Some(node) => {
                // We can deallocate this
                let mut ptr = node;
                loop {
                    let next = unsafe { (*ptr).next };
                    free(ptr);

                    if next.is_some() {
                        ptr = next.unwrap();
                    } else {
                        break;
                    }
                }
            }
        }

        // Clear out all variables
        self.head = None;
        self.tail = None;
        self.blocks = 0;
        self.index = 0;
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

impl IntoIterator for Str {
    type Item = u8;
    type IntoIter = StrIter;

    fn into_iter(self) -> Self::IntoIter {
        return StrIter {
            current: self.head,
            index: 0,
            size: self.index,
        }
    }
}

impl StringOps<Str> for Str {

    /// Split the string into a vector of other strings delimited
    /// by the attribute provided.
    /// 
    /// ```
    /// use teensycore::*;
    /// use teensycore::system::str::*;
    /// 
    /// let string = str!(b"hello\nworld");
    /// let strings = string.split(b'\n');
    /// ```
    fn split(&self, target: u8) -> Vector<Str> {
        let mut result = Vector::new();
        let mut temp = Str::new();

        for char in self.into_iter() {
            if char == target {
                result.push(Str::from_str(&temp));
                temp.clear();
            } else {
                temp.append(&[char]);
            }
        }


        if temp.len() > 0 {
            result.push(Str::from_str(&temp));
        }

        temp.clear();
        temp.drop();
        
        return result;
    }

    /// Searches Self for a matching content string. Returns
    /// true if a match is found.
    fn contains(&self, target: &Str) -> bool {
        return self.index_of(target).is_some();
    }

    fn index_of(&self, target: &Str) -> Option<usize> {
        // Idk waht makes sense for this case
        if target.len() == 0 {
            return Some(0);
        }

        // The algorithm isn't great but it works like this:
        let mut idx = 0;
        let signal = target.char_at(0).unwrap();
        
        for char in self.into_iter() {
            if char == signal {
                // Loop to see if the rest of it matches
                if idx + target.len() > self.len() {
                    return None;
                }
                
                let mut matched = true;
                for r in 0 .. target.len() {
                    if self.char_at(idx + r) != target.char_at(r) {
                        matched = false;
                        break;
                    }
                }

                if matched {
                    return Some(idx);
                }
            }

            idx += 1;
        }

        return None;
    }

    fn reverse(&mut self) -> &mut Self {
        if self.len() == 0 {
            return self;
        }

        // This is going to suck
        let mut tail = self.len() - 1;
        for idx in 0 .. self.len() / 2 {
            let temp = self.char_at(idx).unwrap();
            self.put(idx, self.char_at(tail).unwrap());
            self.put(tail, temp);
            tail -= 1;
        }
        return self;
    }
}


impl PartialEq for Str {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for idx in 0 .. self.len() {
            let left = self.char_at(idx).unwrap();
            let right = other.char_at(idx).unwrap();
            if left != right {
                return false;
            }
        }

        return true;
    }
}

// This might be a bad idea... but it makes BTreeMap super useful
impl PartialOrd for Str {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let min_count = min(self.len(), other.len());
        for idx in 0 .. min_count {
            let left = self.char_at(idx).unwrap();
            let right = other.char_at(idx).unwrap();

            if left == right {
                continue;
            } else {
                return left.partial_cmp(&right);
            }
        }

        // They are the same up to this point
        if self.len() > other.len() {
            return Some(Ordering::Greater);
        } else {
            return Some(Ordering::Less);
        }
    }
}

#[cfg(test)]
mod test_string_builder {

    use super::*;

    fn sb_sb_compare(left: &mut Str, right: &mut Str) {
        assert_eq!(left.len(), right.len());
        for idx in 0 .. left.len() {
            assert_eq!(left.char_at(idx), right.char_at(idx));
        }
    }

    fn sb_equals(sb: Str, text: &[u8]) {
        for i in 0 .. text.len() {
            assert_eq!(sb.char_at(i).unwrap(), text[i]);
        }
    }

    #[test]
    fn test_string_builder() {
        let mut sb = Str::new();
        sb.append(b"hello, world");
        sb.append(b"this has many characters in it. more than 32");

        assert_eq!(sb.blocks, 2);
        assert_eq!(sb.char_at(0), Some(b'h'));
        assert_eq!(sb.char_at(5), Some(b','));
    }

    #[test]
    fn test_capacity() {
        let mut sb = Str::with_capacity(4);
        assert_eq!(sb.append(b"more than 4"), false);
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn test_join() {
        let mut sb = Str::new();
        let mut sb2 = Str::new();

        sb.append(b"hello, ");
        sb2.append(b"world");

        // Join them
        sb.join(&sb2);

        assert_eq!(sb.len(), 12);
    }

    #[test]
    fn test_clear() {
        let mut sb = Str::new();
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
        let mut sb = Str::new();
        sb.append(b"g'morning ");
        sb.append(b"world");

        let slice = sb.slice(2, 8);
        sb_equals(slice, b"morning");

        // Test an extremely large slice, more than a whole page
        sb.append(b"this is more than a whole page of data");
        let slice2 = sb.slice(10, 52);
        sb_equals(slice2, b"worldthis is more than a whole page of data");
    }

    #[test]
    fn test_drop() {
        let mut sb = Str::new();
        sb.append(b"hello, world");
        sb.drop();
        assert_eq!(sb.index, 0);
    }

    #[test]
    fn test_iterator() {
        let mut sb = Str::new();
        let comparator = b"hello, world";
        sb.append(comparator);
        let mut idx = 0;

        for char in sb.into_iter() {
            assert_eq!(comparator[idx], char);
            idx += 1;
        }   
    }

    #[test]
    fn test_contains() {
        let target = str!(b"is a z");
        let target2 = str!(b"this is a test");
        let empty = Str::new();
        let overflow = str!(b"hello world, this is a test of great size");

        let mut sb = Str::new();
        sb.append(b"hello world, this is a test");

        assert_eq!(sb.contains(&target2), true);
        assert_eq!(sb.contains(&target), false);
        assert_eq!(sb.contains(&empty), true);
        assert_eq!(sb.contains(&overflow), false);
    }

    #[test]
    fn test_reverse() {
        sb_sb_compare(str!(b"hello").reverse(), &mut str!(b"olleh"));
        sb_sb_compare(str!(b"helo").reverse(), &mut str!(b"oleh"));
        sb_sb_compare(str!(b"heo").reverse(), &mut str!(b"oeh"));
        sb_sb_compare(str!(b"").reverse(), &mut str!(b""));
    }

    #[test]
    fn test_index_of() {
        let target = str!(b"world");
        let not_found = str!(b"worldz");
        let overflow = str!(b"hello my world, this is not a test");
        let sb = str!(b"hello, world!");

        assert_eq!(sb.index_of(&target), Some(7));
        assert_eq!(sb.index_of(&not_found), None);
        assert_eq!(sb.index_of(&overflow), None);
    }

    #[test]
    fn test_split() {
        let target = str!(b"hello:world");
        let strs = target.split(b':');
        
        sb_sb_compare(&mut strs.get(0).unwrap(), &mut str!(b"hello"));
        sb_sb_compare(&mut strs.get(1).unwrap(), &mut str!(b"world"));
    }
}