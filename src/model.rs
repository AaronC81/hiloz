use super::logic;
use super::script_engine as se;

use std::sync::Arc;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PinDefinition {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Pin {
    pub definition: Arc<PinDefinition>,
    pub value: logic::Value,
    pub pull: logic::Value,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PinConnection {
    component_idx: usize,
    pin_idx: usize
}

#[derive(Debug, Clone)]
pub struct Connection {
    pins: Vec<PinConnection>,
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
    pub pins: Vec<Arc<PinDefinition>>,
    pub variables: Vec<Arc<VariableDefinition>>,
    pub constructor: Option<Arc<se::Function>>,
    pub functions: Vec<Arc<se::Function>>,
    pub script: Arc<se::Function>,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub definition: Arc<ComponentDefinition>,
    pub pins: Vec<Pin>,
    pub variables: Vec<Variable>,

    pub constructor_arguments: Vec<se::Object>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub component_definitions: Vec<Arc<ComponentDefinition>>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub interpreters: Vec<se::Interpreter>,
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
    pub component_idx: usize,
    pub description: ComponentStateModificationDescription,
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
    pub connections: Vec<Connection>,
    modifications: Vec<ComponentStateModification>,
}

impl ComponentIntermediateState {
    pub fn modify(&mut self, modification: ComponentStateModification) {
        self.modifications.push(modification.clone());

        modification.description.apply(&mut self.components[modification.component_idx]);
    }
}

impl Model {
    pub fn step(&mut self) {
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

trait ConnectedComponents {
    fn components(&self) -> Vec<Component>;
    fn connections(&self) -> Vec<Connection>;

    fn connection_value(&self, connection: &Connection) -> Option<logic::Value> {
        let mut value_set = HashSet::new();

        for conn in connection.pins.iter() {
            let this_value = self.components()[conn.component_idx].pins[conn.pin_idx].value;
            if this_value != logic::Value::Unknown {
                value_set.insert(this_value);
            }
        }

        if value_set.is_empty() {
            self.connection_pull(connection)
        } else if value_set.len() == 1 {
            value_set.into_iter().next()
        } else {
            None
        }
    }

    fn connection_pull(&self, connection: &Connection) -> Option<logic::Value> {
        let mut pull_set = HashSet::new();

        for conn in connection.pins.iter() {
            let this_pull = self.components()[conn.component_idx].pins[conn.pin_idx].pull;
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

impl ConnectedComponents for Model {
    fn components(&self) -> Vec<Component> { self.components.clone() }
    fn connections(&self) -> Vec<Connection> { self.connections.clone() }
}

impl ConnectedComponents for ComponentIntermediateState {
    fn components(&self) -> Vec<Component> { self.components.clone() }
    fn connections(&self) -> Vec<Connection> { self.connections.clone() }
}
