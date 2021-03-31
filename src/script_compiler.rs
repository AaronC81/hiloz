use std::collections::HashSet;

use crate::model as m;
use crate::script_engine as se;
use crate::parser as p;

pub struct CompilationContext<'a> {
    parent: Option<&'a CompilationContext<'a>>,
    model: Option<&'a m::Model>,
    component_definition: Option<&'a m::ComponentDefinition>,
    locals: HashSet<String>,
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

    pub fn defined_local(&self, name: &String) -> bool {
        self.locals.contains(name) || if let Some(p) = self.parent {
            p.defined_local(name)
        } else {
            false
        }
    }

    pub fn define_local(&mut self, name: &String) {
        self.locals.insert(name.clone());
    }

    fn child(&'a self) -> Self {
        Self {
            parent: Some(self),
            model: None,
            component_definition: None,
            locals: Default::default(),
        }
    }
}

// TODO: bare expressions used as statements should be wrapped in a separate 
// parse node so that we can pop what they leave on the stack, otherwise we
// end up with a "memory leak" of sorts
fn compile(node: &p::Node, context: &mut CompilationContext) -> Result<Vec<se::Instruction>, String> {
    match node {
        p::Node::Body(b) => {
            let mut new_context = context.child();
            b.iter()
                .map(|n| compile(n, &mut new_context))
                .collect::<Result<Vec<_>, _>>()
                .map(|x| x.concat())
        }

        p::Node::Constant(o) =>
            Ok(vec![se::Instruction::Push(o.clone())]),

        p::Node::Sleep(t) =>
            Ok([
                compile(t, context)?,
                vec![se::Instruction::SuspendSleep]
            ].concat()),

        p::Node::Identifier(i) => {
            let pin_idx = context.component_definition().pin_idx(i);
            let local_defined = context.defined_local(i);

            if pin_idx.is_some() && local_defined {
                return Err(format!("there is both a pin and a local variable called {}", i));
            }

            if let Some(pin_idx) = pin_idx {
                Ok(vec![
                    se::Instruction::Push(se::Object::Integer(pin_idx as i64)),
                    se::Instruction::GetOwnComponentIdx,
                    se::Instruction::ReadComponentPin,
                ])
            } else if local_defined {
                Ok(vec![
                    se::Instruction::GetLocal(i.clone()),
                ])
            } else {
                Err(format!("nothing named {}", i))
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
        },

        p::Node::Dump(node) =>
            Ok([
                compile(node, context)?,
                vec![
                    se::Instruction::Dump,
                ]
            ].concat()),

        p::Node::LocalVariableDefinition { name, value } => {
            if context.locals.contains(name) {
                return Err(format!("local named {} is already defined here", name))
            }

            context.define_local(name);

            Ok([
                vec![
                    se::Instruction::DefineLocal(name.clone())
                ],
                if let Some(initial_value) = value {
                    [
                        compile(&initial_value, context)?,
                        vec![
                            se::Instruction::SetLocal(name.clone())
                        ]
                    ].concat()
                } else {
                    vec![]
                },
            ].concat())
        }

        p::Node::LocalVariableAssignment { name, value } => {
            if !context.defined_local(name) {
                return Err(format!("no defined local named {}", name))
            }

            Ok([
                compile(&value, context)?,
                vec![
                    se::Instruction::SetLocal(name.clone())
                ]
            ].concat())
        }
        
        p::Node::Add(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::Add],
            ].concat())
        }
        p::Node::Subtract(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::Subtract],
            ].concat())
        }
        p::Node::Multiply(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::Multiply],
            ].concat())
        }
        p::Node::Divide(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::Divide],
            ].concat())
        }

        _ => unimplemented!()
    }
}

pub fn compile_script(node: &p::Node, model: Option<&m::Model>, component_definition: Option<&m::ComponentDefinition>) -> Result<Vec<se::Instruction>, String> {
    let mut result = compile(node, &mut CompilationContext {
        parent: None,
        model,
        component_definition,
        locals: Default::default(),
    })?;

    // Add final halt
    result.push(se::Instruction::Halt);
    Ok(result)
}
