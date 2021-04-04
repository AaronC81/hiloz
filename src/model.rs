use binary_heap::PeekMut;
use logic::Value;
use se::SuspensionMode;

use super::logic;
use super::script_engine as se;
use super::model_compiler as mc;
use super::parser as p;

use std::{cmp::Ordering, collections::{BinaryHeap, HashMap, VecDeque, binary_heap}, sync::Arc};
use std::collections::HashSet;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PinDefinition {
    pub name: String,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Pin {
    pub definition: Arc<PinDefinition>,
    pub value: logic::Value,
    pub pull: logic::Value,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PinConnection {
    pub component_idx: usize,
    pub pin_idx: usize
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Connection {
    pub pins: Vec<PinConnection>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VariableDefinition {
    pub name: String,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Variable {
    pub definition: Arc<VariableDefinition>,
    pub value: se::Object,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ComponentDefinition {
    pub name: String,
    pub pins: Vec<Arc<PinDefinition>>,
    pub variables: Vec<Arc<VariableDefinition>>,
    pub constructor: Option<Arc<se::Function>>,
    pub functions: Vec<Arc<se::Function>>,
    pub script: Option<Arc<se::Function>>,
}

impl ComponentDefinition {
    pub fn pin_idx(&self, name: &String) -> Option<usize> {
        self.pins
            .iter()
            .position(|pin_def| &pin_def.name == name)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Component {
    pub instance_name: String,
    pub definition: Arc<ComponentDefinition>,
    pub pins: Vec<Pin>,
    pub variables: Vec<Variable>,
    pub dumps: Vec<se::Object>,
    pub constructor_arguments: Vec<se::Object>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TimingQueueEntry {
    interpreter_idx: usize,
    time_remaining: u64,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TriggerListEntry {
    interpreter_idx: usize,
}

impl PartialOrd for TimingQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimingQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time_remaining.cmp(&self.time_remaining)
            .then_with(|| other.interpreter_idx.cmp(&self.interpreter_idx))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub component_definitions: Vec<Arc<ComponentDefinition>>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub interpreters: Vec<se::Interpreter>,

    pub time_elapsed: u64,
    pub suspended_timing_queue: BinaryHeap<TimingQueueEntry>,
    pub suspended_trigger_list: Vec<TriggerListEntry>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Model) -> bool {
        // TODO: Doesn't compare suspended_timing_queue!
        self.component_definitions == other.component_definitions
        && self.components == other.components
        && self.connections == other.connections
        && self.interpreters == other.interpreters
        && self.time_elapsed == other.time_elapsed
    }
}
impl Eq for Model {}

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
        self.description.apply(&mut components[self.component_idx])
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
    },
    Dump(se::Object),
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
            Self::Dump(value) => {
                component.dumps.push(value);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ComponentIntermediateState {
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub modifications: Vec<ComponentStateModification>,
    pub current_component_idx: Option<usize>,
}

impl ComponentIntermediateState {
    pub fn modify(&mut self, modification: ComponentStateModification) {
        self.modifications.push(modification.clone());

        modification.description.apply(&mut self.components[modification.component_idx]);
    }
}

#[derive(Debug, Clone)]
pub enum StepResult {
    Ok(Vec<ComponentStateModification>),
    Halt,
}

impl Model {
    pub fn step(&mut self) -> StepResult {
        // Make a copy of the current state of the component system
        let intermediate_state = ComponentIntermediateState {
            components: self.components.clone(),
            connections: self.connections.clone(),
            ..ComponentIntermediateState::default()
        };
        let mut all_modifications = vec![];

        // Is there no interpreter which can run without unsuspension?
        // (I think there should always be none, but check just in case)
        if !self.interpreters.iter().any(|i| i.can_run()) {
            // If there are no interpreters to unsuspend, and no interpreters
            // which were going to take a step, then the model will never change
            // again
            let first_interpreter_to_unsuspend = if let Some(x) = self.suspended_timing_queue.pop() {
                x
            } else {
                return StepResult::Halt;
            };

            // Unsuspend the soonest interpreters
            let mut next_interpreters_to_unsuspend = vec![
                first_interpreter_to_unsuspend
            ];

            // Are there any at the same time? If so, let's unsuspend those too
            let queue_step_time = next_interpreters_to_unsuspend[0].time_remaining;
            while let Some(entry) = self.suspended_timing_queue.peek_mut() {
                if entry.time_remaining != queue_step_time {
                    break;
                }

                next_interpreters_to_unsuspend.push(entry.clone());
                binary_heap::PeekMut::pop(entry);
            }

            // Actually unsuspend them
            for i in next_interpreters_to_unsuspend {
                self.interpreters[i.interpreter_idx].resume();
            }

            // Advance other items in the timing queue
            // TODO: This is probably _super_ expensive, we should do this better
            self.suspended_timing_queue = self.suspended_timing_queue.iter()
                .map(|item| TimingQueueEntry {
                    time_remaining: item.time_remaining - queue_step_time,
                    ..item.clone()
                })
                .collect::<BinaryHeap<_>>();
                
            // Advance time elapsed
            self.time_elapsed += queue_step_time;
        }

        // Execute all scripts, collecting component modifications
        for (i, interpreter) in self.interpreters.iter_mut().enumerate() {
            // Only execute if the interpreter can run
            if !interpreter.can_run() {
                continue;
            }

            // Customise the state with some component-specific info
            let mut interpreter_state = intermediate_state.clone();
            interpreter_state.current_component_idx = interpreter.component_idx;

            let interpreter_result = interpreter.execute_until_done(&mut interpreter_state);
            all_modifications.append(&mut interpreter_state.modifications);

            // If this interpreter suspended, add to the timing queue
            match interpreter_result {
                se::InterpreterExecutionResult::Suspend(mode) => {
                    match mode {
                        se::SuspensionMode::Sleep(time) => {
                            self.suspended_timing_queue.push(TimingQueueEntry {
                                interpreter_idx: i,
                                time_remaining: time, 
                            })
                        }

                        se::SuspensionMode::Trigger => {
                            self.suspended_trigger_list.push(TriggerListEntry {
                                interpreter_idx: i,
                            })
                        }
                    }
                },
                se::InterpreterExecutionResult::Halt => (),
                se::InterpreterExecutionResult::Err(s) => panic!(s),
            }
        }

        // Save all connection values before applying modifications
        let connection_values_before_modification = self.all_connection_values();

        // Apply modifications to main component list
        for modification in &all_modifications {
            modification.apply(&mut self.components);
        }

        // So that we can determine which interpreters to trigger, find out
        // which connections this step changed
        let connection_values_after_modification = self.all_connection_values();

        // Find which connections changed
        let connection_values_modified =
            connection_values_after_modification.difference(&connection_values_before_modification)
            .map(|(idx, _)| *idx)
            .collect::<HashSet<_>>();
        
        // Look through interpreters suspended on trigger
        let mut interpreters_to_resume = vec![];
        for suspension in self.suspended_trigger_list.clone().into_iter() {
            let interpreter_idx = suspension.interpreter_idx;
            let component_idx = self.interpreters[interpreter_idx].component_idx.unwrap();
            let component = &self.components[component_idx];

            let all_pins_modified_by_component_this_step = all_modifications.iter()
                .filter_map(|modification| match modification {
                    ComponentStateModification {
                        component_idx: modification_component_idx,
                        description: ComponentStateModificationDescription::Pin { idx, .. }
                    } if *modification_component_idx == component_idx
                        => Some(*idx),
                    _ => None,
                })
                .collect::<Vec<_>>();

            // Get all relevant connections from its component
            // We don't look at any pins which were changed this step, as we
            // don't want a component to change a pin and then trigger itself
            // through that pin
            let all_connection_idxs = component.definition.pins
                .iter()
                .enumerate()
                .filter(|(pin_idx, _)| !all_pins_modified_by_component_this_step.contains(pin_idx))
                .map(|(pin_idx, _)| PinConnection { component_idx, pin_idx })
                .map(|pc| self.pin_connection(&pc))
                .filter(|connection_idx| connection_idx.is_some())
                .map(|connection_idx| connection_idx.unwrap())
                .collect::<Vec<usize>>();

            // Are any of them in the set which changed?
            if all_connection_idxs.iter().any(|idx| connection_values_modified.contains(idx))
            {
                // Let's trigger this component
                interpreters_to_resume.push(suspension);
            }
        };
        
        // Resume the interpreters
        // (Doing this after the loop above avoids mutable reference problems)
        for entry in interpreters_to_resume.iter() {
            self.interpreters[entry.interpreter_idx].resume();
        }

        // Remove all of them from the list
        self.suspended_trigger_list.retain(|entry| !interpreters_to_resume.contains(&entry));

        StepResult::Ok(all_modifications)
    }

    pub fn run<F>(&mut self, until_time: u64, mut between_steps: F) where F : FnMut(&Model, &Vec<ComponentStateModification>) {
        loop {
            match self.step() {
                StepResult::Ok(m) => {
                    between_steps(&self, &m);
                    if self.time_elapsed >= until_time {
                        break;
                    }
                }

                StepResult::Halt => break
            };
        }
    }

    pub fn compile(str: String) -> Result<Model, Box<dyn std::error::Error>> {
        let parsed = p::parse(&str)?;
        mc::compile_model(&parsed)
    }
}

pub trait ConnectedComponents {
    fn components(&self) -> &Vec<Component>;
    fn connections(&self) -> &Vec<Connection>;

    fn components_mut(&mut self) -> &mut Vec<Component>;
    fn connections_mut(&mut self) -> &mut Vec<Connection>;

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

    fn pin_connection(&self, pin_connection: &PinConnection) -> Option<usize> {
        self.connections()
            .iter()
            .position(|conn| conn.pins.contains(pin_connection))
    }

    fn pin_value(&self, conn: &PinConnection) -> logic::Value {
        if let Some(c) = self.pin_connection(conn) {
            self.connection_value(&self.connections()[c]).expect("invalid value for connection")
        } else {
            self.components()[conn.component_idx].pins[conn.pin_idx].value
        }
    }

    fn connect_pins(&mut self, pin_connections: &[PinConnection]) {
        // Is any one of these pins already in a connection?
        let mut existing_connection_idxs = vec![];
        let all_connections = self.connections();
        for pin_connection in pin_connections {
            if let Some(conn) = all_connections
                .iter()
                .position(|c| c.pins.contains(pin_connection))
            {
                existing_connection_idxs.push(conn);
                break;
            }
        }

        if !existing_connection_idxs.is_empty() {
            // If there are existing connections, merge them and add the new
            // pins to that
            let mut merged_connection = Connection { pins: vec![] };
            for existing_connection_idx in existing_connection_idxs {
                let existing_connection = self.connections_mut().remove(existing_connection_idx);
                for existing_pin in existing_connection.pins {
                    if !merged_connection.pins.contains(&existing_pin) {
                        merged_connection.pins.push(existing_pin);
                    }
                }
            }
            for new_pin in pin_connections {
                if !merged_connection.pins.contains(new_pin) {
                    merged_connection.pins.push(new_pin.clone());
                }
            }
            self.connections_mut().push(merged_connection);
        } else {
            // If there are no existing connections containing any of these
            // pins, create a new one
            self.connections_mut().push(Connection {
                pins: pin_connections.to_vec(),
            });
        }        
    }

    fn component_idx(&self, name: &String) -> Option<usize> {
        self.components().iter().position(|c| &c.instance_name == name)
    }

    fn all_connection_values(&self) -> HashSet<(usize, Value)> {
        self.connections()
            .iter()
            .enumerate()
            .map(|(i, conn)| (i, self.connection_value(conn).unwrap()))
            .collect()
    }
}

impl ConnectedComponents for Model {
    fn components(&self) -> &Vec<Component> { &self.components }
    fn connections(&self) -> &Vec<Connection> { &self.connections }

    fn components_mut(&mut self) -> &mut Vec<Component> { &mut self.components }
    fn connections_mut(&mut self) -> &mut Vec<Connection> { &mut self.connections }
}

impl ConnectedComponents for ComponentIntermediateState {
    fn components(&self) -> &Vec<Component> { &self.components }
    fn connections(&self) -> &Vec<Connection> { &self.connections }

    fn components_mut(&mut self) -> &mut Vec<Component> { &mut self.components }
    fn connections_mut(&mut self) -> &mut Vec<Connection> { &mut self.connections }
}
