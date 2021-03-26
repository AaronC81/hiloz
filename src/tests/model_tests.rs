use crate::model::*;
use crate::script_engine::*;
use crate::logic::*;

use std::sync::Arc;

#[test]
fn it_can_take_a_step_and_be_modified() {
    let indicator_pin_in = Arc::new(PinDefinition { name: "in".into() });
    let indicator_script = Arc::new(Function {
        body: vec![
            Instruction::Push(Object::LogicValue(Value::High)),
            Instruction::Push(Object::Integer(0)),
            Instruction::Push(Object::Integer(0)),
            Instruction::ModifyComponentPin,
            Instruction::Halt,
        ],
        parameters: vec![],
    });
    let indicator_def = Arc::new(ComponentDefinition {
        constructor: None,
        functions: vec![],
        pins: vec![indicator_pin_in.clone()],
        script: indicator_script.clone(),
        variables: vec![],
    });

    let indicator = Component {
        definition: indicator_def.clone(),
        constructor_arguments: vec![],
        pins: vec![
            Pin {
                definition: indicator_pin_in,
                pull: Value::Unknown,
                value: Value::Unknown,
            }
        ],
        variables: vec![],
    };
    
    let mut model = Model {
        component_definitions: vec![indicator_def],
        components: vec![indicator],
        connections: vec![],
        interpreters: vec![
            Interpreter {
                frames: vec![
                    InterpreterFrame::new(indicator_script)
                ],
                status: InterpreterStatus::Normal,
            }
        ]
    };

    assert_eq!(model.components[0].pins[0].value, Value::Unknown);
    model.step();
    assert_eq!(model.components[0].pins[0].value, Value::High);
}
