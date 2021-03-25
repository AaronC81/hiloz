use super::logic;

use std::{borrow::Borrow, collections::{HashMap, btree_set::Intersection}, sync::Arc};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Object {
    Null,
    LogicValue(logic::Value),
    Integer(i64),
    Boolean(bool),
    Function(Arc<Function>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Function {
    pub parameters: Vec<String>,
    pub body: Vec<Instruction>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Instruction {
    Push(Object),
    Pop,
    Return,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct InterpreterFrame {
    pub function: Arc<Function>,
    pub arguments: Vec<Object>,
    pub locals: HashMap<String, Object>,
    pub stack: Vec<Object>,
    pub ip: usize,
}

impl InterpreterFrame {
    pub fn new(function: Arc<Function>) -> InterpreterFrame {
        InterpreterFrame {
            function,
            arguments: vec![],
            locals: HashMap::new(),
            stack: vec![],
            ip: 0,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ExecutionResult {
    Ok,
    OkReturn,
    OkNewFrame(InterpreterFrame),
    Err(String),
}

impl InterpreterFrame {
    pub fn execute_one_instruction(&mut self) -> ExecutionResult {
        let instruction = &(*self.function).body[self.ip];
        let mut will_increment_ip = true;
        
        let result = match instruction {
            Instruction::Push(obj) => {
                self.stack.push(obj.clone());
                ExecutionResult::Ok
            },

            Instruction::Pop => {
                self.stack.pop();
                ExecutionResult::Ok
            }

            Instruction::Return => {
                ExecutionResult::OkReturn
            }
        };

        if will_increment_ip {
            self.ip += 1;
        }

        result
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct InterpreterState {
    pub frames: Vec<InterpreterFrame>,
}

impl InterpreterState {
    fn current_frame(&mut self) -> &mut InterpreterFrame {
        self.frames.last_mut().expect("no frames")
    }

    pub fn execute_one_instruction(&mut self) {
        let result = self.current_frame().execute_one_instruction();
        match result {
            ExecutionResult::Ok => (),
            ExecutionResult::OkReturn => {
                if self.current_frame().stack.len() != 1 {
                    panic!("returning frame should have exactly one stack item");
                }

                // Get return value
                let return_value = self.current_frame().stack[0].clone();
                
                // Discard top stack frame
                self.frames.pop();

                // Push return value onto the new top stack frame's stack
                self.current_frame().stack.push(return_value);
            },
            ExecutionResult::OkNewFrame(frame) => {
                self.frames.push(frame);
            }
            ExecutionResult::Err(s) => panic!(s),
        }
    }    
}
