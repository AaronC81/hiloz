use super::logic;
use super::model::{
    ComponentStateModification,
    ComponentIntermediateState,
    ComponentStateModificationDescription,
    PinConnection,
    ConnectedComponents,
};

use std::{borrow::Borrow, collections::HashMap, sync::Arc, usize};

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
    Call,
    Halt,
    Suspend(SuspensionMode),

    // Requires the following on the stack (starting at the top):
    //   - Component index, integer
    //   - Pin index, integer
    //   - New pin value, logic value
    ModifyComponentPin,

    //   - Component index, integer
    //   - Pin index, integer
    ReadComponentPin,
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
pub enum SuspensionMode {
    Sleep(u64),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum InstructionExecutionResult {
    Ok,
    OkReturn,
    OkHalt,
    OkSuspend(SuspensionMode),
    OkNewFrame(InterpreterFrame),
    Err(String),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum FrameExecutionResult {
    Ok,
    OkHalt,
    OkSuspend(SuspensionMode),
    Err(String)
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum InterpreterExecutionResult {
    Halt,
    Suspend(SuspensionMode),
    Err(String)
}

impl InterpreterFrame {
    pub fn execute_one_instruction(&mut self, state: &mut ComponentIntermediateState) -> InstructionExecutionResult {
        let instruction = &(*self.function).body[self.ip];
        let mut will_increment_ip = true;
        
        let result = match instruction {
            Instruction::Push(obj) => {
                self.stack.push(obj.clone());
                InstructionExecutionResult::Ok
            }

            Instruction::Pop => {
                self.stack.pop();
                InstructionExecutionResult::Ok
            }

            Instruction::Return => {
                InstructionExecutionResult::OkReturn
            }

            Instruction::Call => {
                let object = self.stack.pop().expect("stack empty");
                if let Object::Function(func_arc) = object {
                    let function = func_arc.borrow();
                    let Function { parameters, .. } = function;

                    // Collect arguments
                    let mut arguments = vec![];
                    for _ in 0..parameters.len() {
                        arguments.push(self.stack.pop().expect("not enough stack items for arguments"));
                    }

                    // Create new frame
                    InstructionExecutionResult::OkNewFrame(InterpreterFrame {
                        arguments,
                        ..InterpreterFrame::new(func_arc)
                    })
                } else {
                    panic!("calling a non-function");
                }
            }

            Instruction::Halt => InstructionExecutionResult::OkHalt,
            Instruction::Suspend(mode) => InstructionExecutionResult::OkSuspend(mode.clone()),

            Instruction::ModifyComponentPin => {
                let component_idx = self.pop_integer(); 
                let pin_idx = self.pop_integer(); 
                let value = self.pop_logic_value();

                state.modify(ComponentStateModification {
                    component_idx: component_idx as usize,
                    description: ComponentStateModificationDescription::Pin {
                        idx: pin_idx as usize,
                        value,
                    },
                });

                InstructionExecutionResult::Ok
            }

            Instruction::ReadComponentPin => {
                let component_idx = self.pop_integer(); 
                let pin_idx = self.pop_integer();

                let pin_value = state.pin_value(&PinConnection {
                    component_idx: component_idx as usize,
                    pin_idx: pin_idx as usize,
                });
                self.stack.push(Object::LogicValue(pin_value));

                InstructionExecutionResult::Ok
            }
        };

        if will_increment_ip {
            self.ip += 1;
        }

        result
    }

    fn pop_integer(&mut self) -> i64 {
        match self.stack.pop() {
            Some(Object::Integer(i)) => i,
            _ => panic!("expected integer on stack"),
        }
    }

    fn pop_logic_value(&mut self) -> logic::Value {
        match self.stack.pop() {
            Some(Object::LogicValue(v)) => v,
            _ => panic!("expected logic value on stack"),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum InterpreterStatus {
    Normal,
    Suspended,
    Halted,
}

impl Default for InterpreterStatus {
    fn default() -> Self {
        InterpreterStatus::Normal
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct Interpreter {
    pub frames: Vec<InterpreterFrame>,
    pub status: InterpreterStatus,
}

impl Interpreter {
    fn current_frame(&mut self) -> &mut InterpreterFrame {
        self.frames.last_mut().expect("no frames")
    }

    pub fn execute_one_instruction(&mut self, state: &mut ComponentIntermediateState) -> FrameExecutionResult {
        let result = self.current_frame().execute_one_instruction(state);
        match result {
            InstructionExecutionResult::Ok => FrameExecutionResult::Ok,
            InstructionExecutionResult::OkHalt => FrameExecutionResult::OkHalt,
            InstructionExecutionResult::OkSuspend(mode) => FrameExecutionResult::OkSuspend(mode),
            InstructionExecutionResult::OkReturn => {
                if self.current_frame().stack.is_empty() {
                    return FrameExecutionResult::Err(
                        "returning frame has empty stack".into()
                    );
                }

                // Get return value
                let return_value = self.current_frame().stack.last().unwrap().clone();
                
                // Discard top stack frame
                self.frames.pop();

                // Push return value onto the new top stack frame's stack
                self.current_frame().stack.push(return_value);

                FrameExecutionResult::Ok
            },
            InstructionExecutionResult::OkNewFrame(frame) => {
                self.frames.push(frame);
                FrameExecutionResult::Ok
            }
            InstructionExecutionResult::Err(s) => FrameExecutionResult::Err(s),
        }
    }

    pub fn execute_until_done(&mut self, state: &mut ComponentIntermediateState) -> InterpreterExecutionResult {
        loop {
            match self.execute_one_instruction(state) {
                FrameExecutionResult::Ok => (),
                FrameExecutionResult::OkSuspend(mode) => {
                    self.status = InterpreterStatus::Suspended;
                    return InterpreterExecutionResult::Suspend(mode)
                }
                FrameExecutionResult::OkHalt => {
                    self.status = InterpreterStatus::Halted;
                    return InterpreterExecutionResult::Halt
                },
                FrameExecutionResult::Err(s) => {
                    self.status = InterpreterStatus::Halted;
                    return InterpreterExecutionResult::Err(s)
                },
            }
        }
    }
}
