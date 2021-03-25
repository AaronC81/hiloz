use crate::script_engine::*;

use std::sync::Arc;

#[test]
fn it_can_work_with_the_stack() {
    let function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            Instruction::Push(Object::Integer(3)),
            Instruction::Push(Object::Boolean(true)),
            Instruction::Pop,
            Instruction::Push(Object::Null),
            Instruction::Pop,
            Instruction::Pop,
        ]
    });

    let mut frame = InterpreterFrame::new(function);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::Boolean(true)]);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::Null]);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![]);
}

#[test]
fn it_can_return() {
    let called_function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            // Some other garbage on the stack
            Instruction::Push(Object::Integer(1)),
            Instruction::Push(Object::Integer(2)),

            // What we actually return
            Instruction::Push(Object::Integer(3)),
            Instruction::Return,
        ]
    });

    let function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            Instruction::Push(Object::Integer(4)),
            Instruction::Push(Object::Function(called_function)),
            Instruction::Call,
            Instruction::Halt,
        ]
    });

    let mut state = InterpreterState::default();
    state.frames.push(InterpreterFrame::new(function));

    state.execute_until_halt();

    assert_eq!(state.frames.len(), 1);
    assert_eq!(state.frames[0].stack, vec![Object::Integer(4), Object::Integer(3)]);
}
