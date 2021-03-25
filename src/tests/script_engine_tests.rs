use crate::script_engine::*;

use std::sync::Arc;

#[test]
fn it_can_work_with_the_stack() {
    let function = Function {
        parameters: vec![],
        body: vec![
            Instruction::Push(Object::Integer(3)),
            Instruction::Push(Object::Boolean(true)),
            Instruction::Pop,
            Instruction::Push(Object::Null),
            Instruction::Pop,
            Instruction::Pop,
        ]
    };

    let mut frame = InterpreterFrame::new(Arc::new(function));

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::Boolean(true)]);

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3), Object::Null]);

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![Object::Integer(3)]);

    assert_eq!(frame.execute_one_instruction(), ExecutionResult::Ok);
    assert_eq!(frame.stack, vec![]);
}
