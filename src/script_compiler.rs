use crate::model as m;
use crate::script_engine as se;
use crate::script_parser as sp;

pub struct CompilationContext<'a> {
    parent: Option<&'a CompilationContext<'a>>,
    model: Option<&'a m::Model>,
    component_idx: Option<usize>,
}

impl<'a> CompilationContext<'a> {
    fn top_context(&self) -> &CompilationContext<'a> {
        match &self.parent {
            Some(parent) => parent.top_context(),
            None => self,
        }
    }

    fn component(&self) -> &m::Component {
        &self
            .top_context()
            .model
            .unwrap()
            .components[self.top_context().component_idx.unwrap()]
    }

    fn component_idx(&self) -> usize {
        self.top_context().component_idx.unwrap()
    }

    fn child(&'a self) -> Self {
        Self {
            parent: Some(self),
            model: None,
            component_idx: None,
        }
    }
}

// TODO: bare expressions used as statements should be wrapped in a separate 
// parse node so that we can pop what they leave on the stack, otherwise we
// end up with a "memory leak" of sorts
fn compile(node: &sp::Node, context: &CompilationContext) -> Result<Vec<se::Instruction>, String> {
    match node {
        sp::Node::Body(b) =>
            b.iter()
                .map(|n| compile(n, &context.child()))
                .collect::<Result<Vec<_>, _>>()
                .map(|x| x.concat()),

        sp::Node::Constant(o) =>
            Ok(vec![se::Instruction::Push(o.clone())]),

        sp::Node::Sleep(t) =>
            Ok([
                compile(t, context)?,
                vec![se::Instruction::SuspendSleep]
            ].concat()),

        sp::Node::Identifier(i) => {
            let pin_idx = context.component().definition.pin_idx(i);

            if let Some(pin_idx) = pin_idx {
                Ok(vec![
                    se::Instruction::Push(se::Object::Integer(pin_idx as i64)),
                    se::Instruction::Push(se::Object::Integer(context.component_idx() as i64)),
                    se::Instruction::ReadComponentPin,
                ])
            } else {
                Err(format!("no pin named {}", i))
            }
        },

        sp::Node::PinAssignment { target, value } => {
            let pin_idx = if let sp::Node::Identifier(i) = &**target {
                if let Some(idx) = context.component().definition.pin_idx(&i) {
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
                    se::Instruction::Push(se::Object::Integer(context.component_idx() as i64)),
                    se::Instruction::ModifyComponentPin,
                ]
            ].concat())
        }

        _ => unimplemented!()
    }
}

pub fn compile_script(node: &sp::Node, model: Option<&m::Model>, component_idx: Option<usize>) -> Result<Vec<se::Instruction>, String> {
    let mut result = compile(node, &CompilationContext {
        parent: None,
        model,
        component_idx,
    })?;

    // Add final halt
    result.push(se::Instruction::Halt);
    Ok(result)
}
