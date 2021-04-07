use std::{sync::Arc, collections::BinaryHeap, error::Error, fmt};

use m::ConnectedComponents;

use crate::model as m;
use crate::script_compiler as sc;
use crate::script_engine as se;
use crate::parser as p;
use crate::logic as l;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ModelCompilerError {
    description: String,
}

impl ModelCompilerError {
    fn new<S>(description: S) -> ModelCompilerError where S : Into<String> {
        ModelCompilerError { description: description.into() }
    }
}

impl fmt::Display for ModelCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Model compiler error: {}", self.description)
    }
}

impl Error for ModelCompilerError {}

fn compile_constant_object(node: &p::Node) -> Result<se::Object, Box<dyn Error>> {
    if let p::Node::Constant(c) = node {
        Ok(c.clone())
    } else {
        Err(Box::new(ModelCompilerError::new(
            format!("cannot evaluate {:?} in this context", node)
        )))
    }
}

fn compile_component_definition(
    node: &p::Node,
    model: &m::Model,
    component_idx: usize,
    component_definition: &mut m::ComponentDefinition,
) -> Result<(), Box<dyn Error>> {
    match node {
        p::Node::Body(b) => {
            for n in b {
                compile_component_definition(n, model, component_idx, component_definition)?
            }
        }

        p::Node::PinDefinition(name) => {
            if component_definition.pins.iter().find(|p| &p.name == name).is_some() {
                return Err(ModelCompilerError::new(
                    format!("duplicate pin name {}", name)
                ).into());
            }

            component_definition.pins.push(Arc::new(m::PinDefinition {
                name: name.clone(),
            }))
        }

        p::Node::ScriptDefinition(script_body) => {
            if component_definition.script.is_some() {
                return Err(ModelCompilerError::new(
                    "cannot specify script twice"
                ).into())
            }

            let instructions = sc::compile_script(
                script_body,
                Some(model),
                Some(component_definition),
                vec![],
            )?;
            component_definition.script = Some(Arc::new(se::Function {
                body: instructions,
                parameters: vec![],
            }));
        }

        p::Node::ConstructorDefinition { parameters, body } => {
            if component_definition.script.is_some() {
                return Err(ModelCompilerError::new(
                    "cannot specify constructor twice"
                ).into())
            }

            let instructions = sc::compile_script(
                body,
                Some(model),
                Some(component_definition),
                parameters.clone(),
            )?;
            component_definition.constructor = Some(Arc::new(se::Function {
                body: instructions,
                parameters: parameters.clone(),
            }));
        }

        _ => unimplemented!("compile component definition child {:?}", node)
    };

    Ok(())
}

fn compile_connection(
    nodes: &Vec<p::Node>,
    model: &m::Model,
) -> Result<Vec<m::PinConnection>, Box<dyn Error>> {
    nodes.iter().map(|node| match node {
        p::Node::Accessor { target: box p::Node::Identifier(component_name), name: box p::Node::Identifier(pin_name) } => {
            let component_idx = model.component_idx(component_name)
                .ok_or(Box::new(ModelCompilerError::new("missing component")))?;
            let pin_idx = model.components[component_idx].definition.pin_idx(pin_name)
                .ok_or(Box::new(ModelCompilerError::new("missing pin")))?;

            Ok(m::PinConnection { component_idx, pin_idx })
        }
        
        _ => Err(ModelCompilerError::new(
            "connection parameters must be of form: instance.pin"
        ).into())
    }).collect()
}

fn compile_model_(node: &p::Node, model: &mut m::Model) -> Result<(), Box<dyn Error>> {
    match node {
        p::Node::Body(nodes) => {
            for child in nodes {
                compile_model_(child, model)?;
            }
        }

        p::Node::ComponentDefinition { name, body } => {
            let mut component_definition = m::ComponentDefinition {
                name: name.clone(),
                constructor: None,
                functions: vec![],
                pins: vec![],
                script: None,
                variables: vec![],
            };

            // The index of a new component will be the length of the
            // current list
            let component_idx = model.component_definitions.len();

            compile_component_definition(
                body, &model, component_idx, &mut component_definition
            )?;
    
            model.component_definitions.push(Arc::new(component_definition));
        }

        p::Node::ComponentInstantiation { instance_name, component_name, arguments } => {
            let definition = model.component_definitions
                .iter()
                .find(|def| &def.name == component_name);

            let definition = if let Some(x) = definition {
                x
            } else {
                return Err(ModelCompilerError::new(
                    format!("no component named {}", component_name)
                ).into());
            };

            // TODO: Might be good to make a Model::instantiate_component(&mut self, ...) to do all this
            model.components.push(m::Component {
                instance_name: instance_name.clone(),
                definition: definition.clone(),
                pins: definition.pins.iter().map(|pin_def| m::Pin {
                    definition: pin_def.clone(),
                    pull: l::Value::Unknown,
                    value: l::Value::Unknown,
                }).collect(),
                variables: vec![],
                dumps: vec![],
            });

            if let Some(function) = definition.constructor.clone() {
                let constructor_arguments = arguments.iter()
                    .map(|a| compile_constant_object(a))
                    .collect::<Result<Vec<_>, _>>()?;

                model.constructor_interpreters.push(se::Interpreter {
                    component_idx: Some(model.components.len() - 1),
                    frames: vec![
                        se::InterpreterFrame {
                            kind: se::InterpreterFrameKind::FunctionTopLevel,
                            arguments: constructor_arguments,
                            function,
                            ip: 0,
                            locals: Default::default(),
                            stack: vec![],
                        }
                    ],
                    status: se::InterpreterStatus::Normal,
                })
            }

            if let Some(function) = definition.script.clone() {
                model.interpreters.push(se::Interpreter {
                    component_idx: Some(model.components.len() - 1),
                    frames: vec![
                        se::InterpreterFrame {
                            kind: se::InterpreterFrameKind::ScriptTopLevel,
                            arguments: vec![],
                            function,
                            ip: 0,
                            locals: Default::default(),
                            stack: vec![],
                        }
                    ],
                    status: se::InterpreterStatus::Normal,
                });
            }
        }

        p::Node::Connect(nodes) => {
            let pins = compile_connection(nodes, &model)?;
            model.connect_pins(&pins[..]);
        }

        p::Node::EndOfInput => (),

        _ => unimplemented!("compile model child {:?}", node),
    };

    Ok(())
}

pub fn compile_model(node: &p::Node) -> Result<m::Model, Box<dyn Error>> {
    let mut model = m::Model {
        component_definitions: vec![],
        components: vec![],
        connections: vec![],
        interpreters: vec![],
        constructor_interpreters: vec![],

        suspended_timing_queue: BinaryHeap::new(),
        suspended_trigger_list: vec![],
        time_elapsed: 0,
    };

    compile_model_(node, &mut model)?;    
    Ok(model)
}
