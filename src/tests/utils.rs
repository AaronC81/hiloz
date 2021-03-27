
use crate::model::*;
use crate::script_engine::*;
use crate::logic::*;

use std::{sync::Arc, collections::BinaryHeap};

pub fn create_model(scripts: Vec<Vec<Instruction>>) -> Model {
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
            name: "ExampleComponent".into(),
            constructor: None,
            functions: vec![],
            pins: vec![pin.clone()],
            script: Some(function.clone()),
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
        interpreters: functions.into_iter().enumerate().map(|(i, func)|
            Interpreter {
                frames: vec![InterpreterFrame::new(func)],
                status: InterpreterStatus::Normal,
                component_idx: Some(i),
            }
        ).collect(),

        time_elapsed: 0,
        suspended_timing_queue: BinaryHeap::new(),
    }
}
