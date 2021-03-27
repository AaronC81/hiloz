use m::Component;

use crate::model as m;
use crate::script_engine as se;
use crate::script_parser as sp;

pub struct CompilationContext<'a> {
    child: Option<Box<CompilationContext<'a>>>,
    component: Option<&'a m::Component>,
}

fn compile(node: &sp::Node, context: &CompilationContext) -> Vec<se::Instruction> {
    // TODO
    vec![]
}

pub fn compile_script(node: &sp::Node, component: Option<&Component>) -> Vec<se::Instruction> {
    let mut result = compile(node, &CompilationContext {
        child: None,
        component,
    });

    // Add final halt
    result.push(se::Instruction::Halt);
    result
}
