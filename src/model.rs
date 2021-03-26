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
    pub definition: Arc<PinDefinition>,
    pub value: logic::Value,
    pub pull: logic::Value,
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
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub definition: Arc<VariableDefinition>,
    pub value: se::Object,
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
}

#[derive(Debug, Clone)]
pub struct Model {
    component_definitions: Vec<Arc<ComponentDefinition>>,
    components: Vec<Component>,
    connections: Vec<Connection>,
    interpreters: Vec<se::Interpreter>,
}

// Scripts can change components, but we want to give the illusion that all
// scripts execute simultaneously. We do it like this:
//   - Copy components into a new ComponentIntermediateState.
//   - Clone this for each interpreter.
//   - When an interpreter executes an instruction which changes a component, it
//     passes the ComponentStateModification to the ComponentIntermediateState.
//     The change applies immediately to the ComponentIntermediateState, and is
//     recorded.
//   - Once every interpreter has run, apply all of the changes to the true
//     list of components.

#[derive(Debug, Clone)]
pub struct ComponentStateModification {
    component_idx: usize,
    description: ComponentStateModificationDescription,
}

impl ComponentStateModification {
    fn apply<T>(&self, components: &mut T) where T : std::ops::IndexMut<usize, Output=Component> {
        self.description.apply(&mut components[self.component_idx]);
    }
}

#[derive(Debug, Clone)]
pub enum ComponentStateModificationDescription {
    Pin {
        idx: usize,
        value: logic::Value,
    },
    Variable {
        idx: usize,
        value: se::Object,
    }
}

impl ComponentStateModificationDescription {
    fn apply(&self, component: &mut Component) {
        match self.clone() {
            Self::Pin { idx, value } => {
                component.pins[idx].value = value;
            }
            Self::Variable { idx, value } => {
                component.variables[idx].value = value;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComponentIntermediateState {
    pub components: Vec<Component>,
    modifications: Vec<ComponentStateModification>,
}

impl ComponentIntermediateState {
    fn modify(&mut self, modification: ComponentStateModification) {
        modification.description.apply(&mut self.components[modification.component_idx]);
    }
}

impl Model {
    fn step(&mut self) {
        // Make a copy of the current state of the component system
        let intermediate_state = ComponentIntermediateState {
            components: self.components.clone(),
            ..ComponentIntermediateState::default()
        };
        let mut all_modifications = vec![];

        // Execute all scripts, collecting component modifications
        for interpreter in self.interpreters.iter_mut() {
            let mut interpreter_state = intermediate_state.clone();
            interpreter.execute_until_done(&mut interpreter_state);

            all_modifications.append(&mut interpreter_state.modifications);
        }

        // Apply modifications to main component list
        for modification in all_modifications {
            modification.apply(&mut self.components);
        }
    }
}
