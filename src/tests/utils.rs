
use crate::model::*;
use crate::script_engine::*;
use crate::logic::*;
use crate::parser::*;
use crate::model_compiler::*;

use std::{sync::Arc, collections::BinaryHeap};

pub fn create_model_with_scripts(scripts: Vec<Vec<Instruction>>) -> Model {
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
    for (i, def) in component_definitions.iter().enumerate() {
        components.push(Component {
            instance_name: format!("instance{}", i),
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
            dumps: vec![],
        });
    }
    
    Model {
        component_definitions,
        components,
        interpreters: functions.into_iter().enumerate().map(|(i, func)|
            Interpreter {
                frames: vec![InterpreterFrame::new(func)],
                status: InterpreterStatus::Normal,
                component_idx: Some(i),
            }
        ).collect(),

        ..Default::default()
    }
}

pub fn create_model<S>(contents: S) -> Model where S : Into<String> {
    Model::compile(contents.into()).unwrap()
}
