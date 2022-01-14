use crate::*;
use crate::system::vector::*;
use crate::system::strings::*;
use crate::system::map::*;

pub trait Observable<T> {
    fn emit(&self, key: String, payload: &T);
    fn on(&mut self, key: String, method: fn(&T));
}

pub struct Observer<T> {
    methods: BTreeMap<String, Vector::<fn(&T)>>,
}

impl <T> Observer<T> {
    pub fn new() -> Self {
        return Observer {
            methods: BTreeMap::new(),
        };
    }    
}

impl <T> Observable<T> for Observer<T> {
    fn emit(&self, key: String, payload: &T) {
        match self.methods.get(key) {
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

    fn on(&mut self, key: String, method: fn(&T)) {
        let cur_node = self.methods.get_mut(key);
        match cur_node {
            None => {
                // Insert
                self.methods.insert(key, vector!(method));
            },
            Some(node) => {
                node.push(method);
            }
        }
    }
}


#[cfg(test)]
mod test { 
    use super::*;
    
    #[test]
    fn test_observable() {
        let mut observer = Observer::<String>::new();
        observer.on(vec_str!(b"update_clock"), |_| {
            assert_eq!(true, true);
            return;
        });
        observer.on(vec_str!(b"update_clock"), |_| {
            assert_eq!(true, true);
            return;
        });

        // This is a kinda bad test but, trust me, it works...
        observer.emit(vec_str!(b"update_clock"), &vec_str!(b"1234567"));

    }
}