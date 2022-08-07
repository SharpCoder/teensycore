//! A basic linked-list implementation
//! which supports push/pop/enqueue/dequeue
//! as well as random reads and puts.
//! 
//! It is loosely modeled after the 
//! JavaScript array.
#![allow(dead_code)]
use crate::{mem::{ alloc, free }, math::rand};
use core::iter::{Iterator};

/// This macro returns a vector of the items you pass to it.
#[macro_export]
macro_rules! vector {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = Vector::new();
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}


/// This macro takes a static string and returns
/// a vector containing the sequence of characters.
#[macro_export]
macro_rules! vec_str {
    ($arr:tt) => {
        Vector::from_slice($arr)
    };
}

pub trait Stack <T> {
    fn push(&mut self, item: T);
    fn pop(&mut self) -> Option<T>;
}

pub trait Queue <T> {
    fn enqueue(&mut self, item: T);
    fn dequeue(&mut self) -> Option<T>;
}

pub trait Array<T> {
    fn get(&self, index: usize) -> Option<T>;
    fn get_mut(&mut self, index: usize) -> Option<&mut T>;
    fn put(&mut self, index: usize, element: T);
    fn size(&self) -> usize;
}

/**
Vector is a heap-backed datastructure
which allocates dynamic memory and implements Stack.
*/
#[derive(Copy, Clone)]
pub struct Node<T : Clone + Copy> {
    pub item: T,
    pub next: Option<*mut Node<T>>,
}

pub struct Vector<T : Clone + Copy> {
    pub head: Option<*mut Node<T>>,
    pub size: usize,
}

pub struct NodeIter<T: Clone+Copy> {
    current: Option<Node<T>>,
    index: usize,
    size: usize,
}

impl <T: Clone+Copy> Iterator for NodeIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            None => {
                return None;
            },
            Some(element) => {
                let result = element.item;
                
                // Check if we have next
                if element.next.is_none() {
                    self.current = None;
                } else {
                    self.current = Some(unsafe { *(element.next.unwrap()) });
                }

                self.index += 1;

                // Check if we are technically beyond the size
                if self.index > self.size {
                    return None;
                }

                return Some(result);
            }
        };
    }
}

impl <T: Clone + Copy> Clone for Vector<T> {
    fn clone(&self) -> Self {
        if self.head.is_none() {
            return Vector::new();
        }
        
        let mut result = Vector::new();
        let mut ptr = self.head.unwrap();

        loop {
            let item = unsafe { (*ptr).item };
            result.enqueue(item);

            if unsafe { ptr.as_mut().unwrap() }.next.is_some() {
                ptr = unsafe { ptr.as_mut().unwrap() }.next.unwrap();
            } else {
                break;
            }
        }

        return result;
    }
}

impl <T: Clone + Copy> Copy for Vector<T> {
    
}

impl <T: Clone + Copy> Array<T> for Vector<T> {
    fn size(&self) -> usize {
        return self.size;
    }

    fn put(&mut self, index: usize, element: T) {
        let node = self.get_mut(index);
        match node {
            None => {},
            Some(el) => {
                (*el) = element;
            }
        }
    }

    fn get(&self, index: usize) -> Option<T> {
        if self.head.is_none() || index >= self.size {
            return None;
        } else {
            // Travel n times through the linked list
            let mut ptr = self.head.unwrap();
            for _ in 0 .. index {
                ptr = unsafe { ptr.as_mut().unwrap() }.next.unwrap();
            }
            return unsafe { Some((*ptr).item) };
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.head.is_none() || index >= self.size {
            return None;
        } else {
            // Travel n times through the linked list
            let mut ptr = self.head.unwrap();
            for _ in 0 .. index {
                ptr = unsafe { ptr.as_mut().unwrap() }.next.unwrap();
            }
            return unsafe { Some(&mut (*ptr).item) };
        }
    }
}

impl <T: Clone + Copy> Queue<T> for Vector<T> {
    fn enqueue(&mut self, item: T) {
        // Add it to the end of the stack
        let ptr = alloc();
        unsafe {
            (*ptr) = Node {
                item: item,
                next: None,
            }
        }

        if self.head.is_none() {
            self.head = Some(ptr);
        } else {
            let mut tail_ptr = self.head.unwrap();
    
            // Find the tail
            while unsafe { tail_ptr.as_mut().unwrap() }.next.is_some() {
                tail_ptr = unsafe { (*tail_ptr).next.unwrap() };
            }
    
            unsafe { (*tail_ptr).next = Some(ptr) };
        }
        self.size += 1;

    }

    fn dequeue(&mut self) -> Option<T> {
        match self.head {
            None => {
                return None;
            },
            Some(node) => {
                // Copy the reference
                let node_item = unsafe { node.as_mut().unwrap() };
                
                // Free the actual node.
                free(node);

                let result = node_item.item;
                self.head = node_item.next;
                self.size = self.size - 1;
                return Some(result);
            },
        }; 
    }
}

impl <T: Clone + Copy> Stack<T> for Vector<T> {
    fn push(&mut self, item: T) {
        self.enqueue(item);
    }

    fn pop(&mut self) -> Option<T> {
        if self.head.is_none() {
            return None;
        }

        let node_item;

        if self.size == 1 {
            // Return head node
            node_item = unsafe { (*(self.head.unwrap())).item };
            // Free the head
            free(self.head.unwrap());
            self.head = None;

        } else {
            // Travel to the correct node
            let mut ptr = self.head.unwrap();
            for _ in 0 .. (self.size() - 2) {
                ptr = unsafe { (*ptr).next.unwrap() };
            }
            
            node_item = unsafe { (*(*ptr).next.unwrap()).item };
            unsafe {
                // Free the node
                free((*ptr).next.unwrap());
                // Update node parent to point at nothing 
                (*ptr).next = None 
            };
        }

        self.size -= 1;
        return Some(node_item);
    }
}
impl <T: Clone + Copy> Vector<T> {
    pub fn new() -> Self {
        return Vector { head: None, size: 0 };
    }

    pub fn into_iter(&self) -> NodeIter<T> {
        if self.head.is_none() {
            return NodeIter {
                current: None,
                size: 0,
                index: 0,
            };
        } else {
            return NodeIter {
                current: Some(unsafe { *self.head.unwrap() }),
                size: self.size(),
                index: 0,
            };
        }
    }

    pub fn from_slice(items: &[T]) -> Self {
        let mut result = Vector::new();
        for item in items {
            result.enqueue(*item);
        }
        return result;
    }

    pub fn size(&self) -> usize {
        return self.size;
    }

    pub fn join(&mut self, vec_to_join: &Vector<T>) -> &mut Self {
        let mut copy = vec_to_join.clone();
        for _ in 0 .. vec_to_join.size() {
            self.enqueue(copy.dequeue().unwrap());
        }
        copy.clear();
        return self;
    }

    pub fn substr(&self, start: usize, length: usize) -> Option<Self> {
        let mut result = Vector::new();
        if start + length > self.size() {
            return None;
        }

        for idx in start .. (start + length) {
            result.enqueue(self.get(idx).unwrap());
        }

        return Some(result);
    }

    pub fn reverse(&self) -> Vector::<T> {
        let mut result = Vector::new();
        for idx in 0 .. self.size() {
            result.push(self.get(self.size() - idx - 1).unwrap());
        }
        return result;
    }

    /// This method will take a vector of <T>
    /// and return a copy of it, shuffled.
    /// 
    /// This is supposed to use the fisher-yates algorithm.
    /// 
    /// ```
    /// use teensycore::system::vector::*;
    /// use teensycore::*;
    /// 
    /// let vec = vector!(1,2,3,4);
    /// let shuffled = vec.shuffle();
    /// ```
    pub fn shuffle(&self) -> Vector::<T> {
        let mut result = self.reverse();
        
        // Items of 0 or 1 size do not need shuffled.
        if result.size() < 2 {
            return result;
        }

        for idx in 0 .. (self.size() - 2) {
            let random_idx = idx + (rand() % (self.size() - idx) as u64) as usize;
            
            let rand_val = result.get(random_idx).unwrap();
            let orig_val = result.get(idx).unwrap();

            result.put(idx, rand_val);
            result.put(random_idx, orig_val);
        }

        return result;
    }

    pub fn clear(&mut self) {
        while self.size() > 0 {
            self.pop();
        }
    }

    // Alias method for clear
    // just to make it obvious whats happening.
    pub fn free(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod test { 
    use crate::math::seed_rand;

    use super::*;

    #[derive(Copy, Clone)]
    pub struct ShadowVec {
        pub items: Vector::<u8>,
        pub time: usize,
    }

    fn test_reverse() {
        let vec = vector!(1u8, 2, 3, 4, 5);
        let reversed = vec.reverse();

        assert_eq!(reversed.get(0), Some(5));
        assert_eq!(reversed.get(1), Some(4));
        assert_eq!(reversed.get(2), Some(3));
        assert_eq!(reversed.get(3), Some(2));
        assert_eq!(reversed.get(4), Some(1));

        let reversed2 = vec.reverse();
        assert_eq!(reversed2.get(0), Some(1));
        assert_eq!(reversed2.get(1), Some(2));
        assert_eq!(reversed2.get(2), Some(3));
        assert_eq!(reversed2.get(3), Some(4));

    }

    #[test]
    fn advanced_copy() {
        let shadow = ShadowVec {
            items: Vector::from_slice(&[1,2,3,4,5]),
            time: 1337,
        };

        let next = shadow.clone();
        assert_eq!(next.items.size(), 5);
    }

    #[test]
    fn stack() {
        let mut list = Vector::new();
        list.push(32);
        list.push(64);
        list.push(128);
        list.push(256);

        assert_eq!(list.size(), 4);
        assert_eq!(list.pop(), Some(256));
        assert_eq!(list.size(), 3);
        assert_eq!(list.pop(), Some(128));
        assert_eq!(list.size(), 2);
        assert_eq!(list.pop(), Some(64));
        assert_eq!(list.size(), 1);
        assert_eq!(list.pop(), Some(32));
        assert_eq!(list.size(), 0);
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn stack_get() {
        let mut list = Vector::<u32>::new();
        list.enqueue(32);
        list.enqueue(64);
        list.enqueue(128);
        list.enqueue(256);
        list.enqueue(512);

        assert_eq!(list.get(0), Some(32));
        assert_eq!(list.get(1), Some(64));
        assert_eq!(list.get(3), Some(256));
        assert_eq!(list.get(2), Some(128));
        assert_eq!(list.get(4), Some(512));
        assert_eq!(list.get(5), None);
        assert_eq!(list.get(100), None);

        let list2 = Vector::<i32>::new();
        assert_eq!(list2.get(0), None);
        assert_eq!(list2.get(100), None);
    }

    #[test]
    fn test_stack_clone() {
        let list = Vector::from_slice(&[32, 64, 128, 256, 512]);
        let mut cloned_list = list.clone();
        assert_eq!(cloned_list.pop(), Some(512));
        assert_eq!(cloned_list.pop(), Some(256));
        assert_eq!(cloned_list.pop(), Some(128));
        assert_eq!(cloned_list.pop(), Some(64));
        assert_eq!(cloned_list.pop(), Some(32));
        assert_eq!(cloned_list.pop(), None);

        cloned_list.join(&Vector::from_slice(&[32,64]));
        let mut list3 = cloned_list.clone();
        list3.join(&Vector::from_slice(&[128]));
        assert_eq!(list3.get(0), Some(32));
    }

    #[test]
    fn test_vector_queue() {
        let mut list = Vector::new();
        list.enqueue(32);
        list.enqueue(64);
        list.enqueue(128);
        
        assert_eq!(list.dequeue(), Some(32));
        assert_eq!(list.dequeue(), Some(64));
        assert_eq!(list.dequeue(), Some(128));
        assert_eq!(list.dequeue(), None);
    }

    #[test]
    fn test_vector_join() {
        let mut list1 = Vector::from_slice(&[32,64,128]);
        let list2 = Vector::from_slice(&[256,512]);
        
        list1.join(&list2);

        assert_eq!(list1.pop(), Some(512));
        assert_eq!(list1.pop(), Some(256));
    }

    #[test]
    fn test_vector_insert() {
        let mut vec = Vector::from_slice(&[1,2,3,4,5]);
        vec.put(3, 100);

        let mut found = false;
        for idx in 0 .. vec.size() {
            if vec.get(idx) == Some(100) {
                found = true;
            }
        }
        assert_eq!(found, true);
    }

    #[test]
    fn test_iterator() {
        let vec = vector!(1,2,3,4);
        let mut count = 0;
        for _ in vec.into_iter() {
            count += 1;
        }

        assert_eq!(count, vec.size());
    }

    #[test]
    fn test_shuffle() {
        let vec = vector!(1,2,3,4,5,6);
        seed_rand(1340);

        let next_vec = vec.shuffle();
        assert_eq!(next_vec.get(0).unwrap(), 4);
        assert_eq!(next_vec.get(1).unwrap(), 3);
        assert_eq!(next_vec.get(2).unwrap(), 6);
        assert_eq!(next_vec.get(3).unwrap(), 1);
        assert_eq!(next_vec.get(4).unwrap(), 2);
        assert_eq!(next_vec.get(5).unwrap(), 5);
    }
}