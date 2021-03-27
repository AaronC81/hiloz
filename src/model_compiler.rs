use std::{sync::Arc, collections::BinaryHeap};

use m::PinDefinition;

use crate::model as m;
use crate::script_compiler as sc;
use crate::script_engine as se;
use crate::script_parser as sp;

fn compile_component_definition(
    node: &sp::Node,
    model: &m::Model,
    component_idx: usize,
    component_definition: &mut m::ComponentDefinition,
) -> Result<(), String> {
    match node {
        sp::Node::Body(b) => {
            for n in b {
                compile_component_definition(n, model, component_idx, component_definition)?
            }
        }

        sp::Node::PinDefinition(name) => {
            if component_definition.pins.iter().find(|p| &p.name == name).is_some() {
                return Err(format!("duplicate pin name {}", name));
            }

            component_definition.pins.push(Arc::new(m::PinDefinition {
                name: name.clone(),
            }))
        }

        sp::Node::ScriptDefinition(script_body) => {
            if component_definition.script.is_some() {
                return Err("cannot specify script twice".into())
            }

            let instructions = sc::compile_script(
                script_body,
                Some(model),
                Some(component_idx), // FIXME
            )?;
            component_definition.script = Some(Arc::new(se::Function {
                body: instructions,
                parameters: vec![],
            }));
        }

        _ => unimplemented!("compile component definition child {:?}", node)
    };

    Ok(())
}

pub fn compile_model(node: &sp::Node) -> Result<m::Model, String> {
    let mut model = m::Model {
        component_definitions: vec![],
        components: vec![],
        connections: vec![],
        interpreters: vec![],

        suspended_timing_queue: BinaryHeap::new(),
        time_elapsed: 0,
    };

    if let sp::Node::Body(b) = node {
        for n in b {
            match n {
                sp::Node::ComponentDefinition { name, body } => {
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

                _ => unimplemented!("compile model child {:?}", n),
            };
        }
    } else {
        return Err("expected body node for model compilation".into())
    }
    
    Ok(model)
}
