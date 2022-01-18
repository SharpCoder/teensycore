//! Box provides a possibly unecessary level of abstraction

#[derive(Copy, Clone)]
pub struct Box {
    item: *const u32,
}

impl Box {
    pub fn new<T>(item: T) -> Self {
        // Get a reference to the thing
        let ptr = &item as *const T;
        return Box {
            item: ptr as *const u32,
        }
    }

    pub fn unbox<T>(&self) -> *const T {
        // let ptr = unsafe { self.item as *const T };
        return unsafe { self.item as *const T };
    }

    pub fn from_raw<T: Copy>(item: Self) -> T {
        return unsafe { *item.unbox::<T>() };
    }
}