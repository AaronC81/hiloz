use std::{collections::HashSet, env::var, process};

use se::Instruction;

use crate::model as m;
use crate::script_engine as se;
use crate::parser as p;

fn replace_instruction<F>(
    instructions: &Vec<se::Instruction>,
    sought: se::Instruction,
    process_fn: F
) -> Vec<se::Instruction>
where F : Fn(usize, se::Instruction) -> se::Instruction
{
    instructions
        .iter()
        .enumerate()
        .map(|(idx, inst)| {
            let inst = inst.clone();
            if inst == sought {
                process_fn(idx, inst)
            } else {
                inst
            }
        })
        .collect()
}

pub struct CompilationContext<'a> {
    parent: Option<&'a CompilationContext<'a>>,
    model: Option<&'a m::Model>,
    component_definition: Option<&'a m::ComponentDefinition>,
    locals: HashSet<String>,
    parameters: Vec<String>,
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

    pub fn defined_parameter(&self, name: &String) -> bool {
        self.parameter_idx(name).is_some()
    }

    pub fn defined_component_variable(&mut self, name: &String) -> bool {
        self.component_definition().variables.iter()
            .any(|var_def| &var_def.name == name)
    }

    pub fn parameter_idx(&self, name: &String) -> Option<usize> {
        self.parameters.iter().position(|x| x == name).or_else(||
            if let Some(p) = self.parent {
                p.parameter_idx(name)
            } else {
                None
            })
    }

    fn child(&'a self) -> Self {
        Self {
            parent: Some(self),
            model: None,
            component_definition: None,
            locals: Default::default(),
            parameters: vec![],
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

        p::Node::Trigger => Ok(vec![se::Instruction::SuspendTrigger]),

        p::Node::Identifier(i) => {
            let pin_idx = context.component_definition().pin_idx(i);
            let local_defined = context.defined_local(i);
            let component_variable_defined = context.defined_component_variable(i);
            let parameter_idx = context.parameter_idx(i);

            if [pin_idx.is_some(), local_defined, component_variable_defined, parameter_idx.is_some()].iter()
                .filter(|x| **x)
                .count() > 1
            {
                return Err(format!("there are multiple items called {}", i));
            }

            if let Some(pin_idx) = pin_idx {
                Ok(vec![
                    se::Instruction::Push(se::Object::Integer(pin_idx as i64)),
                    se::Instruction::GetOwnComponentIdx,
                    se::Instruction::ReadComponentPin,
                ])
            } else if let Some(parameter_idx) = parameter_idx {
                Ok(vec![
                    se::Instruction::GetParameter(parameter_idx),
                ])
            } else if local_defined || component_variable_defined {
                Ok(vec![
                    se::Instruction::GetVariable(i.clone()),
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
                            se::Instruction::SetVariable(name.clone())
                        ]
                    ].concat()
                } else {
                    vec![]
                },
            ].concat())
        }

        p::Node::LocalVariableAssignment { name, value } => {
            if !context.defined_local(name) && !context.defined_component_variable(name) {
                return Err(format!("no defined variable named {}", name))
            }

            Ok([
                compile(&value, context)?,
                vec![
                    se::Instruction::SetVariable(name.clone())
                ]
            ].concat())
        }
        
        p::Node::LogicNot(box x) => {
            Ok([
                compile(x, context)?,
                vec![se::Instruction::LogicNot]
            ].concat())
        }
        p::Node::LogicAnd(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::LogicAnd],
            ].concat())
        }
        p::Node::LogicOr(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::LogicOr],
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
        p::Node::Equal(box a, box b) => {
            Ok([
                compile(b, context)?,
                compile(a, context)?,
                vec![se::Instruction::Equal],
            ].concat())
        }

        p::Node::Loop(box inner) => {
            let inner_instructions = compile(inner, context)?;
            let jump_distance = -(inner_instructions.len() as i64);

            // Replace MagicBreak with a jump of the correct distance
            let inner_instructions = replace_instruction(
                &inner_instructions,
                se::Instruction::MagicBreak,
                |i, _| {
                    // +1 jumps over the jump back to the beginning
                    let break_jump_distance = inner_instructions.len() - i + 1;
                    se::Instruction::Jump(break_jump_distance as i64)
                }
            );

            Ok([
                inner_instructions,
                vec![se::Instruction::Jump(jump_distance)],
            ].concat())
        }
        p::Node::If { condition: box condition, body: box body } => {
            // We don't implement "else" yet, so all we need to do is jump over
            // the body if the condition is not met
            let condition_instructions = compile(condition, context)?;
            let body_instructions = compile(body, context)?;
            let jump_distance = (body_instructions.len() as i64) + 1;

            Ok([
                condition_instructions,
                vec![
                    se::Instruction::LogicNot,
                    se::Instruction::JumpConditional(jump_distance),
                ],
                body_instructions,
            ].concat())
        }
        p::Node::Break => {
            Ok(vec![se::Instruction::MagicBreak])
        }

        _ => unimplemented!()
    }
}

pub fn compile_script(node: &p::Node, model: Option<&m::Model>, component_definition: Option<&m::ComponentDefinition>, parameters: Vec<String>) -> Result<Vec<se::Instruction>, String> {
    let mut result = compile(node, &mut CompilationContext {
        parent: None,
        model,
        component_definition,
        locals: Default::default(),
        parameters,
    })?;

    // Add final halt
    result.push(se::Instruction::Halt);
    Ok(result)
}
