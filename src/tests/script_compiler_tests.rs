use crate::script_compiler::*;
use crate::script_parser::*;
use crate::script_engine::Instruction;
use crate::script_engine::Object::*;
use crate::script_parser::Node::*;
use crate::logic::*;
use super::utils;

#[test]
fn it_compiles_a_blank_body_to_halt() {
    assert_eq!(
        compile_script(&script_block().parse(b"{}").unwrap(), None, None),
        Ok(vec![Instruction::Halt])
    );
}

#[test]
fn it_compiles_pin_assignments() {
    let model = utils::create_model(vec![vec![], vec![]]);

    assert_eq!(
        compile_script(&script_block().parse(b"{
            pin <- H;
            pin <- L;
        }").unwrap(), Some(&model), Some(1)),
        Ok(vec![
            Instruction::Push(LogicValue(Value::High)),
            Instruction::Push(Integer(0)),
            Instruction::Push(Integer(1)),
            Instruction::ModifyComponentPin,

            Instruction::Push(LogicValue(Value::Low)),
            Instruction::Push(Integer(0)),
            Instruction::Push(Integer(1)),
            Instruction::ModifyComponentPin,

            Instruction::Halt
        ])
    );

    // Test when named pin doesn't exist
    assert!(matches!(
        compile_script(&script_block().parse(b"{
            pin_that_does_not_exist <- H;
        }").unwrap(), Some(&model), Some(1)),
        Err(_)
    ));
}
