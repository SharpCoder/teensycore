use crate::system::vector::*;

/**
Buffer is a data structure that supports
stack and queue operations, but is
a fixed length and does not use extra
memory.
*/
pub struct Buffer<const SIZE: usize, T> {
    pub data: [T; SIZE],
    pub tail: usize,
}

impl <const SIZE: usize, T : Copy> Stack<T> for Buffer<SIZE, T> {
    fn push(&mut self, item: T) {
        if self.tail == SIZE {
            // Discard the data. we are buffer oerflow.
            return;
        }
        
        self.data[self.tail] = item;
        self.tail += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.tail == 0 {
            return None;
        }

        let item = self.data[self.tail - 1];
        self.tail -= 1;
        return Some(item);
    }
}

impl <const SIZE: usize, T : Copy> Queue<T> for Buffer<SIZE, T> {
    fn enqueue(&mut self, item: T) {
        if self.tail == SIZE {
            // Discard the data. we are buffer oerflow.
            return;
        }

        self.data[self.tail] = item;
        self.tail += 1;
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.tail == 0 {
            return None;
        }

        let result = self.data[0];

        // Shift everything to the left
        for idx in 0 .. self.tail {
            self.data[idx] = self.data[idx + 1].clone();
        }

        self.tail -= 1;

        return Some(result);
    }
}

impl Array<u8> for &[u8] {
    fn size(&self) -> usize {
        return self.len();
    }

    fn get(&self, index: usize) -> Option<u8> {
        if index >= self.len() {
            return None;
        }
        return Some(self[index]);
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut u8> {
        if index >= self.len() {
            return None;
        }

        panic!("Not implemented");
    }

    fn put(&mut self, _index: usize, _element: u8) {
        panic!("Not implemented");
    }
}

impl <const SIZE: usize, T : Copy> Array<T> for Buffer<SIZE, T> {
    fn size(&self) -> usize {
        return self.tail;
    }

    fn get(&self, index: usize) -> Option<T> {
        if index >= self.tail {
            return None;
        } else {
            return Some(self.data[index]);
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.tail {
            return None;
        } else {
            return Some(&mut self.data[index]);
        }
    }

    fn put(&mut self, index: usize, element: T) {
        self.data.as_mut()[index] = element;
    }
}

impl <const SIZE: usize, T : Copy> Buffer<SIZE, T> {
    pub fn new(default: T) -> Self {
        return Buffer {
            data: [default; SIZE],
            tail: 0,
        }
    }

    pub fn size(&self) -> usize {
        return self.tail;
    }

    pub fn as_array(&self) -> &[T] {
        return &self.data[..];
    }

    pub fn clear(&mut self) {
        self.tail = 0;
    }
}





#[cfg(test)]
mod test { 
    use super::*;

    #[test]
    fn buffer() {
        let mut buffer = Buffer::<10, u8>::new(0);
        buffer.enqueue(32);
        buffer.enqueue(64);
        buffer.enqueue(128);
        assert_eq!(buffer.dequeue(), Some(32));
        assert_eq!(buffer.dequeue(), Some(64));
        assert_eq!(buffer.dequeue(), Some(128));
        assert_eq!(buffer.dequeue(), None);

        buffer.enqueue(32);
        buffer.enqueue(64);
        buffer.push(128);
        assert_eq!(buffer.pop(), Some(128));
        assert_eq!(buffer.pop(), Some(64));
        assert_eq!(buffer.pop(), Some(32));
    }
}