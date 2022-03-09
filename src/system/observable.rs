use crate::*;
use crate::system::vector::*;
use crate::system::str::*;
use crate::system::map::*;

pub struct Observable<'a, T> {
    methods: BTreeMap<Str, Vector::<&'a dyn Fn(&T)>>,
}

impl <'a, T> Observable<'a, T> {
    pub fn new() -> Self {
        return Observable {
            methods: BTreeMap::new(),
        };
    }    

    pub fn emit(&self, key: &Str, payload: &T) {
        match self.methods.get(&key) {
            None => {
                // Nobody cares
            },
            Some(methods) => {
                for idx in 0 .. methods.size() {
                    methods.get(idx).unwrap()(payload);
                }
            }
        }
    }

    pub fn on(&mut self, key: &Str, method: &'a dyn Fn(&T)) {
        let cur_node = self.methods.get_mut(&key);
        match cur_node {
            None => {
                // Insert
                self.methods.insert(Str::from_str(key), vector!(method));
            },
            Some(node) => {
                node.push(method);
            }
        }
    }
}


#[cfg(test)]
mod test { 
    use crate::system::boxed::Box;

    use super::*;
    static mut CALLED: usize = 0;

    #[test]
    fn test_observable() {
        unsafe { CALLED = 0; }
        
        let mut observer = Observable::<Str>::new();
        observer.on(&str!(b"update_clock"), &|_| {
            assert_eq!(true, true);
            unsafe { CALLED += 1; }
            return;
        });
        observer.on(&str!(b"update_clock"), &|_| {
            assert_eq!(true, true);
            unsafe { CALLED += 1; }
            return;
        });

        // This is a kinda bad test but, trust me, it works...
        observer.emit(&str!(b"update_clock"), &str!(b"1234567"));
        assert_eq!(unsafe { CALLED }, 2);
    }
}