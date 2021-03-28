use binary_heap::PeekMut;

use super::logic;
use super::script_engine as se;
use super::model_compiler as mc;
use super::parser as p;

use std::{cmp::Ordering, collections::{BinaryHeap, VecDeque, binary_heap}, sync::Arc};
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
    pins: Vec<PinConnection>,
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

    pub constructor_arguments: Vec<se::Object>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TimingQueueEntry {
    interpreter_idx: usize,
    time_remaining: u64,
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

#[derive(Debug, Clone)]
pub struct Model {
    pub component_definitions: Vec<Arc<ComponentDefinition>>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    pub interpreters: Vec<se::Interpreter>,

    pub time_elapsed: u64,
    pub suspended_timing_queue: BinaryHeap<TimingQueueEntry>,
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
    pub current_component_idx: Option<usize>,
}

impl ComponentIntermediateState {
    pub fn modify(&mut self, modification: ComponentStateModification) {
        self.modifications.push(modification.clone());

        modification.description.apply(&mut self.components[modification.component_idx]);
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum StepResult {
    Ok,
    Halt,
}

impl Model {
    pub fn step(&mut self) -> StepResult {
        // Make a copy of the current state of the component system
        let intermediate_state = ComponentIntermediateState {
            components: self.components.clone(),
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
                    }
                },
                se::InterpreterExecutionResult::Halt => (),
                se::InterpreterExecutionResult::Err(s) => panic!(s),
            }
        }

        // Apply modifications to main component list
        for modification in all_modifications {
            modification.apply(&mut self.components);
        }

        StepResult::Ok
    }
}

pub trait ConnectedComponents {
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

    fn compile(str: String) -> Result<Model, Box<dyn std::error::Error>> {
        let parsed = p::top_level().parse(str.as_bytes())?;
        mc::compile_model(&parsed)
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
