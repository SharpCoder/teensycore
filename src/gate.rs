/*
Gating is what allows tasks in the RTOS to execute 
quickly. Think of it like a thread mixed with a promise. 
The gating mechanism defines a flow governed by a 
condition statemenet and then a function delegate. 
If the condition is not met, the gate will yield 
until a later point.
*/
use crate::clock::*;
use crate::system::vector::*;

type CondFn = fn(&mut Gate) -> bool;
type ExecFn = fn();

#[derive(Copy, Clone)]
pub struct Gate {
    pub conditions: Vector::<CondFn>,
    pub functions: Vector::<ExecFn>,
    pub durations: Vector::<uNano>,
    pub target_times: Vector::<uNano>,
    pub current_index: usize,
    pub tail: usize,
    pub sealed: bool,
    pub compiled: bool,
}

#[macro_export]
macro_rules! gate_open {
    ( $( $x:expr ),* ) => {
        {
            let id = code_hash();
            let current_node = unsafe { GATES.get(&id) };
            let result: &mut Gate;
            
            match current_node {
                None => {
                    // Let's create a new gate.
                    let new_gate = crate::mem::alloc::<Gate>();
                    unsafe { *new_gate = Gate::new(); }

                    // This new gate is what we'l return
                    result = unsafe { &mut (*new_gate) };

                    // Insert the gate in the global gate register
                    unsafe { GATES.insert(id, new_gate as u32) };
                },
                Some(gate) => {
                    result = unsafe { ((*gate) as *mut Gate).as_mut().unwrap() };
                }
            }

            result
        }
    };
}


impl Gate {
    pub fn new() -> Gate {
        return Gate {
            conditions: Vector::new(),
            functions: Vector::new(),
            durations: Vector::new(),
            target_times: Vector::new(),
            current_index: 0usize,
            tail: 0usize,
            sealed: false,
            compiled: false,
        };
    }

    pub fn when(&mut self, cond: CondFn, then: ExecFn) -> &mut Self {
        if self.compiled {
            return self;
        }

        self.target_times.push(0);
        self.durations.push(0);
        self.conditions.push(cond);
        self.functions.push(then);
        self.tail += 1;
        return self;
    }

    pub fn when_nano(&mut self, duration_nanos: uNano, then: ExecFn) -> &mut Self {
        if self.compiled {
            return self;
        }

        self.target_times.push(0);
        self.durations.push(duration_nanos);
        self.conditions.push(when_cond);
        self.functions.push(then);
        self.tail += 1;
        return self;
    }

    /// If called, this gate will only ever execute one time.
    pub fn once(&mut self, func: ExecFn) -> &mut Self {
        if self.compiled {
            return self;
        }

        func();
        return self;
    }

    /// If a gate is sealed, it will only execute to completion once.
    /// After that, it will remain idle forever.
    pub fn sealed(&mut self) -> &mut Self {
        if self.compiled {
            return self;
        }

        self.sealed = true;
        return self;
    }
    
    /// Return the compiled gate, ready to be processed.
    pub fn compile(&mut self) -> Gate {

        // debug_u32(unsafe { GATES.size() } as u32, b"gate size");
        // debug_u64(self.id, b" working with gate");
        if self.compiled {
            self.process();
        } else {
            self.compiled = true;
        }

        return *self;
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
        self.conditions.clear();
        self.functions.clear();
        self.target_times.clear();
        self.durations.clear();
        self.compiled = false;
    }

    // This method will evaluate the current
    // gate condition and, if true, execute
    // the underlying block.
    pub fn process(&mut self) {

        // Check if it is valid to process
        if self.current_index == self.tail && self.sealed {
            return;
        }

        let cond = self.conditions.get(self.current_index).unwrap();
        let then = self.functions.get(self.current_index).unwrap();

        if cond(self) {
            then();
            self.current_index += 1;

            if self.current_index == self.tail && self.sealed {
                return;
            } else if self.current_index == self.tail {
                self.current_index = 0;
            } 

            self.target_times.put(self.current_index, nanos() + self.durations.get(self.current_index).unwrap());
        }

    }
}

fn when_cond(gate: &mut Gate) -> bool {
    return nanos() > gate.target_times.get(gate.current_index).unwrap();
}