use crate::script_compiler::*;
use crate::script_parser::*;
use crate::script_engine::Instruction;
use crate::script_engine::Object::*;
use crate::script_parser::Node::*;
use crate::logic::*;

#[test]
fn it_compiles_a_blank_body_to_halt() {
    assert_eq!(compile_script(&Body(vec![]), None), vec![Instruction::Halt]);
}
