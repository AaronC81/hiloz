use crate::script_compiler::*;
use crate::parser;
use crate::script_engine::Instruction;
use crate::script_engine::Object::*;
use crate::parser::*;
use crate::logic::*;
use super::utils;

fn parse_block(input: &str) -> Node {
    parse_rule(input, Rule::statement_block).unwrap()
}

#[test]
fn it_compiles_a_blank_body_to_halt() {
    assert_eq!(
        compile_script(&parse_block("{}"), None, None),
        Ok(vec![Instruction::Halt])
    );
}

#[test]
fn it_compiles_pin_assignments() {
    let model = utils::create_model_with_scripts(vec![vec![], vec![]]);

    assert_eq!(
        compile_script(&parse_block("{
            pin <- H;
            pin <- L;
        }"), Some(&model), Some(&model.component_definitions[1])),
        Ok(vec![
            Instruction::Push(LogicValue(Value::High)),
            Instruction::Push(Integer(0)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Push(LogicValue(Value::Low)),
            Instruction::Push(Integer(0)),
            Instruction::GetOwnComponentIdx,
            Instruction::ModifyComponentPin,

            Instruction::Halt
        ])
    );

    // Test when named pin doesn't exist
    assert!(matches!(
        compile_script(&parse_block("{
            pin_that_does_not_exist <- H;
        }"), Some(&model), Some(&model.component_definitions[1])),
        Err(_)
    ));
}

#[test]
fn it_compiles_loop_statements() {
    let model = utils::create_model_with_scripts(vec![vec![]]);

    assert_eq!(
        compile_script(&parse_block("{
            loop {
                _dump(1);
            }
        }"), Some(&model), Some(&model.component_definitions[0])),
        Ok(vec![
            Instruction::Push(Integer(1)),
            Instruction::Dump,
            Instruction::Jump(-2),

            Instruction::Halt
        ])
    );
}

#[test]
fn it_compiles_break_statements() {
    let model = utils::create_model_with_scripts(vec![vec![]]);

    assert_eq!(
        compile_script(&parse_block("{
            loop {
                _dump(1);
                break;
                _dump(2);
                _dump(3);
            }
        }"), Some(&model), Some(&model.component_definitions[0])),
        Ok(vec![
            Instruction::Push(Integer(1)),
            Instruction::Dump,

            Instruction::Jump(6),

            Instruction::Push(Integer(2)),
            Instruction::Dump,

            Instruction::Push(Integer(3)),
            Instruction::Dump,

            Instruction::Jump(-7),

            Instruction::Halt
        ])
    );
}
