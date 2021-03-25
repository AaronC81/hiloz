use super::logic;
use super::script_engine as se;

use std::sync::Arc;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PinDefinition {
    name: String,
}

#[derive(Debug, Clone)]
pub struct Pin {
    definition: Arc<PinDefinition>,
    value: logic::Value,
    pull: logic::Value,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pins: Vec<Arc<Pin>>,
}

impl Connection {
    fn overall_pull(&self) -> Option<logic::Value> {
        let mut pull_set = HashSet::new();

        for pin in self.pins.iter() {
            let this_pull = pin.pull;
            if this_pull != logic::Value::Unknown {
                pull_set.insert(this_pull);
            }
        }

        if pull_set.is_empty() {
            Some(logic::Value::Unknown)
        } else if pull_set.len() == 1 {
            pull_set.into_iter().next()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct VariableDefinition {
    name: String,
}

#[derive(Debug, Clone)]
pub struct Variable {
    definition: Arc<VariableDefinition>,
    value: se::Object,
}

#[derive(Debug, Clone)]
pub struct ComponentDefinition {
    pins: Vec<PinDefinition>,
    variables: Vec<VariableDefinition>,
    constructor: Option<se::Function>,
    functions: Vec<se::Function>,
    script: se::Function,
    subcomponents: Box<Component>,
}

#[derive(Debug, Clone)]
pub struct Component {
    definition: Arc<ComponentDefinition>,
    pins: Vec<Pin>,
    variables: Vec<Variable>,

    constructor_arguments: Vec<se::Object>,

    // TODO: script state
}

#[derive(Debug, Clone)]
pub struct Model {
    component_definitions: Vec<Arc<ComponentDefinition>>,
    top_level_component_instances: Vec<Component>,
    connections: Vec<Connection>,
}
