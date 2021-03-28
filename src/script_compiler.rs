use crate::model as m;
use crate::script_engine as se;
use crate::parser as p;

pub struct CompilationContext<'a> {
    parent: Option<&'a CompilationContext<'a>>,
    model: Option<&'a m::Model>,
    component_definition: Option<&'a m::ComponentDefinition>,
}

impl<'a> CompilationContext<'a> {
    fn top_context(&self) -> &CompilationContext<'a> {
        match &self.parent {
            Some(parent) => parent.top_context(),
            None => self,
        }
    }

    fn component_definition(&self) -> &m::ComponentDefinition {
        self.top_context().component_definition.unwrap()
    }

    fn child(&'a self) -> Self {
        Self {
            parent: Some(self),
            model: None,
            component_definition: None,
        }
    }
}

// TODO: bare expressions used as statements should be wrapped in a separate 
// parse node so that we can pop what they leave on the stack, otherwise we
// end up with a "memory leak" of sorts
fn compile(node: &p::Node, context: &CompilationContext) -> Result<Vec<se::Instruction>, String> {
    match node {
        p::Node::Body(b) =>
            b.iter()
                .map(|n| compile(n, &context.child()))
                .collect::<Result<Vec<_>, _>>()
                .map(|x| x.concat()),

        p::Node::Constant(o) =>
            Ok(vec![se::Instruction::Push(o.clone())]),

        p::Node::Sleep(t) =>
            Ok([
                compile(t, context)?,
                vec![se::Instruction::SuspendSleep]
            ].concat()),

        p::Node::Identifier(i) => {
            let pin_idx = context.component_definition().pin_idx(i);

            if let Some(pin_idx) = pin_idx {
                Ok(vec![
                    se::Instruction::Push(se::Object::Integer(pin_idx as i64)),
                    se::Instruction::GetOwnComponentIdx,
                    se::Instruction::ReadComponentPin,
                ])
            } else {
                Err(format!("no pin named {}", i))
            }
        },

        p::Node::PinAssignment { target, value } => {
            let pin_idx = if let p::Node::Identifier(i) = &**target {
                if let Some(idx) = context.component_definition().pin_idx(&i) {
                    idx
                } else {
                    return Err(format!("no pin named {}", i))
                }
            } else {
                return Err("can only assign to pin".into())
            };

            Ok([
                compile(value, context)?,
                vec![
                    se::Instruction::Push(se::Object::Integer(pin_idx as i64)),
                    se::Instruction::GetOwnComponentIdx,
                    se::Instruction::ModifyComponentPin,
                ]
            ].concat())
        }

        _ => unimplemented!()
    }
}

pub fn compile_script(node: &p::Node, model: Option<&m::Model>, component_definition: Option<&m::ComponentDefinition>) -> Result<Vec<se::Instruction>, String> {
    let mut result = compile(node, &CompilationContext {
        parent: None,
        model,
        component_definition,
    })?;

    // Add final halt
    result.push(se::Instruction::Halt);
    Ok(result)
}
