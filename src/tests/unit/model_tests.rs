use crate::model::*;
use crate::script_engine::*;
use crate::logic::*;
use super::utils;

#[test]
fn it_can_read_its_pin_state() {
    let mut model = utils::create_model_with_scripts(vec![vec![
        // Read pin, should be X
        Instruction::Push(Object::Integer(0)),
        Instruction::Push(Object::Integer(0)),
        Instruction::ReadComponentPin,

        // Set pin to H
        Instruction::Push(Object::LogicValue(Value::High)),
        Instruction::Push(Object::Integer(0)),
        Instruction::Push(Object::Integer(0)),
        Instruction::ModifyComponentPin,

        // Read pin, should be H
        Instruction::Push(Object::Integer(0)),
        Instruction::Push(Object::Integer(0)),
        Instruction::ReadComponentPin,

        Instruction::Halt,
    ]]);

    assert_eq!(model.components[0].pins[0].value, Value::Unknown);
    model.step();
    assert_eq!(model.components[0].pins[0].value, Value::High);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::LogicValue(Value::Unknown),
        Object::LogicValue(Value::High),
    ]);
}

#[test]
fn it_can_be_suspended_for_a_time_delay() {
    let mut model = utils::create_model_with_scripts(vec![vec![
        Instruction::Push(Object::Integer(1)),
        Instruction::Push(Object::Integer(1000)),
        Instruction::SuspendSleep,

        Instruction::Push(Object::Integer(2)),
        Instruction::Push(Object::Integer(2000)),
        Instruction::SuspendSleep,

        Instruction::Push(Object::Integer(3)),
        Instruction::Push(Object::Integer(3000)),
        Instruction::SuspendSleep,

        Instruction::Push(Object::Integer(4)),
        Instruction::Push(Object::Integer(0)),
        Instruction::SuspendSleep,

        Instruction::Halt,
    ]]);

    assert!(model.interpreters[0].can_run());
    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 1);
    assert_eq!(model.time_elapsed, 0);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
    ]);

    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 1);
    assert_eq!(model.time_elapsed, 1000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
    ]);
    
    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 1);
    assert_eq!(model.time_elapsed, 3000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
        Object::Integer(3),
    ]);

    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 1);
    assert_eq!(model.time_elapsed, 6000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
        Object::Integer(3),
        Object::Integer(4),
    ]);

    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 0);
    assert_eq!(model.time_elapsed, 6000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
        Object::Integer(3),
        Object::Integer(4),
    ]);
    assert_eq!(model.interpreters[0].status, InterpreterStatus::Halted);
}

#[test]
fn it_can_resume_multiple_interpreters_after_time_delay() {
    let mut model = utils::create_model_with_scripts(vec![
        vec![
            Instruction::Push(Object::Integer(1)),
            Instruction::Push(Object::Integer(1000)),
            Instruction::SuspendSleep,

            Instruction::Push(Object::Integer(2)),
            Instruction::Push(Object::Integer(2000)),
            Instruction::SuspendSleep,

            Instruction::Push(Object::Integer(3)),
            Instruction::Halt,
        ],
        vec![
            Instruction::Push(Object::Integer(1)),
            Instruction::Push(Object::Integer(3000)),
            Instruction::SuspendSleep,

            Instruction::Push(Object::Integer(2)),
            Instruction::Halt,
        ],
    ]);

    assert!(model.interpreters[0].can_run());
    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 2);
    assert_eq!(model.time_elapsed, 0);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
    ]);
    assert_eq!(model.interpreters[1].frames[0].stack, vec![
        Object::Integer(1),
    ]);

    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 2);
    assert_eq!(model.time_elapsed, 1000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
    ]);
    assert_eq!(model.interpreters[1].frames[0].stack, vec![
        Object::Integer(1),
    ]);

    model.step();
    assert_eq!(model.suspended_timing_queue.len(), 0);
    assert_eq!(model.time_elapsed, 3000);
    assert_eq!(model.interpreters[0].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
        Object::Integer(3),
    ]);
    assert_eq!(model.interpreters[1].frames[0].stack, vec![
        Object::Integer(1),
        Object::Integer(2),
    ]);
    assert_eq!(model.interpreters[0].status, InterpreterStatus::Halted);
    assert_eq!(model.interpreters[1].status, InterpreterStatus::Halted);
}
