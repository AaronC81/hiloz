use std::{sync::Arc, collections::BinaryHeap};

use m::PinDefinition;

use crate::script_compiler::*;
use crate::script_parser::*;
use crate::script_engine::Instruction;
use crate::script_engine::Object::*;
use crate::script_engine as se;
use crate::script_parser::Node::*;
use crate::model_compiler::*;
use crate::model::*;
use crate::model as m;
use crate::logic::*;
use super::utils;

#[test]
fn it_compiles_a_model() {
    assert_eq!(
        compile_model(&top_level().parse(b"
            define component Indicator {
                pin a;
                pin b;

                script {

                }
            }
        ").unwrap()),
        Ok(Model {
            component_definitions: vec![
                Arc::new(m::ComponentDefinition {
                    name: "Indicator".into(),
                    constructor: None,
                    functions: vec![],
                    pins: vec![
                        Arc::new(m::PinDefinition { name: "a".into() }),
                        Arc::new(m::PinDefinition { name: "b".into() }),
                    ],
                    script: Some(Arc::new(se::Function {
                        parameters: vec![],
                        body: vec![
                            Instruction::Halt,
                        ],
                    })),
                    variables: vec![],
                }),
            ],
            components: vec![],
            connections: vec![],
            interpreters: vec![],
            suspended_timing_queue: BinaryHeap::new(),
            time_elapsed: 0,
        })
    )
}
