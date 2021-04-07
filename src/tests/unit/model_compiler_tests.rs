use std::{collections::{BinaryHeap, HashMap}, sync::Arc};

use m::PinDefinition;

use crate::script_compiler::*;
use crate::parser;
use crate::script_engine::Instruction;
use crate::script_engine::Object::*;
use crate::script_engine as se;
use crate::parser::*;
use crate::model_compiler::*;
use crate::model::*;
use crate::model as m;
use crate::logic::*;
use super::utils;

#[test]
fn it_compiles_a_model() {
    let function = Arc::new(se::Function {
        parameters: vec![],
        body: vec![
            Instruction::Push(LogicValue(Value::High)),
            Instruction::Push(Integer(0)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Push(LogicValue(Value::Low)),
            Instruction::Push(Integer(1)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Push(Integer(1000)),
            Instruction::SuspendSleep,

            Instruction::Push(LogicValue(Value::Low)),
            Instruction::Push(Integer(0)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Push(LogicValue(Value::High)),
            Instruction::Push(Integer(1)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Push(Integer(1000)),
            Instruction::SuspendSleep,

            Instruction::Halt,
        ],
    });

    let component_def = Arc::new(m::ComponentDefinition {
        name: "Indicator".into(),
        constructor: None,
        functions: vec![],
        pins: vec![
            Arc::new(m::PinDefinition { name: "a".into() }),
            Arc::new(m::PinDefinition { name: "b".into() }),
        ],
        script: Some(function.clone()),
        variables: vec![],
    });

    assert_eq!(
        Model::compile("
            define component Indicator {
                pin a;
                pin b;

                script {
                    a <- H;
                    b <- L;
                    sleep(1000);
                    a <- L;
                    b <- H;
                    sleep(1000);
                }
            }

            component first_instance = Indicator();
            component second_instance = Indicator();
        ".into()).unwrap(),
        Model {
            component_definitions: vec![
                component_def.clone()
            ],
            components: vec![
                m::Component {
                    definition: component_def.clone(),
                    pins: vec![
                        m::Pin {
                            definition: component_def.clone().pins[0].clone(),
                            pull: Value::Unknown,
                            value: Value::Unknown,
                        },
                        m::Pin {
                            definition: component_def.clone().pins[1].clone(),
                            pull: Value::Unknown,
                            value: Value::Unknown,
                        },
                    ],
                    instance_name: "first_instance".into(),
                    variables: vec![],
                    dumps: vec![],
                },

                m::Component {
                    definition: component_def.clone(),
                    pins: vec![
                        m::Pin {
                            definition: component_def.clone().pins[0].clone(),
                            pull: Value::Unknown,
                            value: Value::Unknown,
                        },
                        m::Pin {
                            definition: component_def.clone().pins[1].clone(),
                            pull: Value::Unknown,
                            value: Value::Unknown,
                        },
                    ],
                    instance_name: "second_instance".into(),
                    variables: vec![],
                    dumps: vec![],
                }
            ],
            connections: vec![],
            interpreters: vec![
                se::Interpreter {
                    component_idx: Some(0),
                    frames: vec![
                        se::InterpreterFrame {
                            kind: se::InterpreterFrameKind::ScriptTopLevel,
                            arguments: vec![],
                            ip: 0,
                            locals: HashMap::new(),
                            stack: vec![],
                            function: function.clone(),
                        }
                    ],
                    status: se::InterpreterStatus::Normal,
                },
                se::Interpreter {
                    component_idx: Some(1),
                    frames: vec![
                        se::InterpreterFrame {
                            kind: se::InterpreterFrameKind::ScriptTopLevel,
                            arguments: vec![],
                            ip: 0,
                            locals: HashMap::new(),
                            stack: vec![],
                            function: function.clone(),
                        }
                    ],
                    status: se::InterpreterStatus::Normal,
                },
            ],
            ..Default::default()
        }
    )
}
