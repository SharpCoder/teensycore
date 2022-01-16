//! Closure allows you to store a reference to a function,
//! along with a single argument that will be passed to the function.
//! 
//! This is useful if you want to store a lits of callbacks, for example,
//! and bind them to some argument that exists outside their scope.
//! 
//! ```no-test
//! use teensycore::system::vector::*;
//! use teensycore::system::closure::*;
//! 
//! let mut list = Vector::new();
//! list.push(Closure::bind(&invoke, ("hello", 42)));
//! list.push(Closure::bind(&invoke, ("world!", 32)));
//! 
//! for closure in list.into_iter() {
//!     closure.invoke();
//! }
//! 
//! fn invoke(args: (&str, i32)) {
//!     println!("{} {}", args.0, args.1);
//! }
//! ```
#[derive(Copy, Clone)]
pub struct Closure<'a, T : Copy> {
    method: &'a dyn Fn(T),
    arg: T,
}

impl <'a, T: Copy> Closure<'a, T> {
    pub fn bind(method: &'a dyn Fn(T), arg: T) -> Self {
        return Closure {
            method: method,
            arg: arg,
        };
    }

    pub fn invoke(&self) {
        (self.method)(self.arg);
    }
}

#[cfg(test)]
mod test_closure {
    use super::*;
    use crate::system::vector::*;

    #[test]
    fn test() {
        let mut list = Vector::new();
        list.push(Closure::bind(&invoke, ("hello", 42)));
        list.push(Closure::bind(&invoke, ("world!", 32)));
        for closure in list.into_iter() {
            closure.invoke();
        }

        // If we get here, it didn't crash. That's a win!
        assert_eq!(true, true);
    }

    fn invoke(message: (&str, i32)) {
        std::println!("{}", message.0);
    }
}