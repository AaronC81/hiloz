use crate::{model::ComponentIntermediateState, model::ComponentStateModificationDescription, script_engine::*};
use crate::logic::Value;

use std::sync::Arc;

#[test]
fn it_can_work_with_the_stack() {
    let mut comp_state = ComponentIntermediateState::default();

    let function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            Instruction::Push(Object::Integer(3)),
            Instruction::Push(Object::LogicValue(Value::High)),
            Instruction::Pop,
            Instruction::Push(Object::Null),
            Instruction::Pop,
            Instruction::Pop,
        ]
    });

    let mut frame = InterpreterFrame::new(function);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::LogicValue(Value::High)]);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::Null]);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(&mut comp_state), InstructionExecutionResult::Ok);
    assert_eq!(frame.stack, vec![]);
}

#[test]
fn it_can_return() {
    let mut comp_state = ComponentIntermediateState::default();

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

    let mut state = Interpreter::default();
    state.frames.push(InterpreterFrame::new(function));

    assert_eq!(state.execute_until_done(&mut comp_state), InterpreterExecutionResult::Halt);

    assert_eq!(state.frames.len(), 1);
    assert_eq!(state.frames[0].stack, vec![Object::Integer(4), Object::Integer(3)]);
}

#[test]
fn it_can_have_locals() {
    let mut comp_state = ComponentIntermediateState::default();

    let function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            Instruction::DefineLocal("a".into()),
            Instruction::DefineLocal("b".into()),

            Instruction::Push(Object::Integer(3)),
            Instruction::Push(Object::Integer(5)),
            Instruction::SetLocal("a".into()),
            Instruction::SetLocal("b".into()),

            Instruction::GetLocal("a".into()),
            Instruction::GetLocal("a".into()),
            Instruction::GetLocal("b".into()),

            Instruction::Halt,
        ]
    });

    let mut state = Interpreter::default();
    state.frames.push(InterpreterFrame::new(function));

    assert_eq!(state.execute_until_done(&mut comp_state), InterpreterExecutionResult::Halt);

    assert_eq!(state.frames.len(), 1);
    assert_eq!(state.frames[0].stack, vec![
        Object::Integer(5),
        Object::Integer(5),
        Object::Integer(3),
    ]);
}

#[test]
fn it_performs_comparisons_and_jumps() {
    let mut comp_state = ComponentIntermediateState::default();

    let function = Arc::new(Function {
        parameters: vec![],
        body: vec![
            // Set up loop counter
            Instruction::Push(Object::Integer(1)),

            // Loop start - comparison and conditional jump
            Instruction::Duplicate,
            Instruction::Push(Object::Integer(10)),
            Instruction::Equal,
            Instruction::JumpConditional(4),

            // No loop body here

            // Increment counter
            Instruction::Push(Object::Integer(1)),
            Instruction::Add,
            Instruction::Jump(-6),

            Instruction::Halt,
        ]
    });

    let mut state = Interpreter::default();
    state.frames.push(InterpreterFrame::new(function));

    assert_eq!(state.execute_until_done(&mut comp_state), InterpreterExecutionResult::Halt);

    let dumps = comp_state
        .modifications
        .iter()
        .map(|m| match m.description.clone() {
            ComponentStateModificationDescription::Dump(obj) => obj,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    assert_eq!(state.frames.len(), 1);
    assert_eq!(state.frames[0].stack, vec![
        Object::Integer(10),
    ]);
}
