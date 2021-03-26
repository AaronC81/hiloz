use crate::model::*;
use crate::script_engine::*;
use crate::logic::*;

use std::{collections::BinaryHeap, sync::Arc};

fn create_model(scripts: Vec<Vec<Instruction>>) -> Model {
    let functions = scripts.into_iter()
        .map(|body| Arc::new(Function {
            body,
            parameters: vec![],
        }))
        .collect::<Vec<_>>();

    let mut component_definitions = vec![];
    for function in &functions {
        let pin = Arc::new(PinDefinition { name: "pin".into() });
        component_definitions.push(Arc::new(ComponentDefinition {
            constructor: None,
            functions: vec![],
            pins: vec![pin.clone()],
            script: function.clone(),
            variables: vec![],
        }))
    }

    let mut components = vec![];
    for def in &component_definitions {
        components.push(Component {
            definition: def.clone(),
            constructor_arguments: vec![],
            pins: vec![
                Pin {
                    definition: def.pins[0].clone(),
                    pull: Value::Unknown,
                    value: Value::Unknown,
                }
            ],
            variables: vec![],
        });
    }
    
    Model {
        component_definitions,
        components,
        connections: vec![],
        interpreters: functions.into_iter().map(|func|
            Interpreter {
                frames: vec![InterpreterFrame::new(func)],
                status: InterpreterStatus::Normal,
            }
        ).collect(),

        time_elapsed: 0,
        suspended_timing_queue: BinaryHeap::new(),
    }
}

#[test]
fn it_can_take_a_step_and_be_modified() {
    let mut model = create_model(vec![vec![
        Instruction::Push(Object::LogicValue(Value::High)),
        Instruction::Push(Object::Integer(0)),
        Instruction::Push(Object::Integer(0)),
        Instruction::ModifyComponentPin,
        Instruction::Halt,
    ]]);

    assert_eq!(model.components[0].pins[0].value, Value::Unknown);
    model.step();
    assert_eq!(model.components[0].pins[0].value, Value::High);
}

#[test]
fn it_can_read_its_pin_state() {
    let mut model = create_model(vec![vec![
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
    let mut model = create_model(vec![vec![
        Instruction::Push(Object::Integer(1)),
        Instruction::Suspend(SuspensionMode::Sleep(1000)),

        Instruction::Push(Object::Integer(2)),
        Instruction::Suspend(SuspensionMode::Sleep(2000)),

        Instruction::Push(Object::Integer(3)),
        Instruction::Suspend(SuspensionMode::Sleep(3000)),

        Instruction::Push(Object::Integer(4)),
        Instruction::Suspend(SuspensionMode::Sleep(0)),

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
    let mut model = create_model(vec![
        vec![
            Instruction::Push(Object::Integer(1)),
            Instruction::Suspend(SuspensionMode::Sleep(1000)),

            Instruction::Push(Object::Integer(2)),
            Instruction::Suspend(SuspensionMode::Sleep(2000)),

            Instruction::Push(Object::Integer(3)),
            Instruction::Halt,
        ],
        vec![
            Instruction::Push(Object::Integer(1)),
            Instruction::Suspend(SuspensionMode::Sleep(3000)),

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
