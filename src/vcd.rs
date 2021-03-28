use crate::model as m;
use crate::logic as l;

#[derive(Default)]
pub struct VcdGenerator {
    lines: Vec<String>,
}

impl VcdGenerator {
    fn to_var_identifier(&self, component_idx: usize, pin_idx: usize) -> String {
        format!("c{}p{}", component_idx, pin_idx)
    }

    fn logic_to_symbol(&self, value: l::Value) -> char {
        match value {
            l::Value::High => '1',
            l::Value::Low => '0',
            l::Value::Unknown => 'x',
        }
    }

    fn add<S>(&mut self, line: S) where S : Into<String> {
        self.lines.push(line.into())
    }

    pub fn generate_header(&mut self, model: &m::Model) {
        // TODO: customisable time unit, maybe in model?
        self.add("$timescale 1ms $end");

        self.add("$scope module simulation $end");
        for (component_idx, component) in model.components.iter().enumerate() {
            self.add(format!("$scope module {} $end", component.instance_name));
            for (pin_idx, pin) in component.definition.pins.iter().enumerate() {
                self.add(format!(
                    "$var wire 1 {} {} $end",
                    self.to_var_identifier(component_idx, pin_idx),
                    component.definition.pins[pin_idx].name,
                ))
            }
            self.add("$upscope $end");
        }
        self.add("$upscope $end");
        self.add("$enddefinitions $end");
    }

    pub fn step(&mut self, model: &m::Model, modifications: &Vec<m::ComponentStateModification>) {
        self.add(format!("#{}", model.time_elapsed));
        for modification in modifications {
            match modification.description {
                m::ComponentStateModificationDescription::Pin { idx: pin_idx, value } => {
                    self.add(format!(
                        "{}{}",
                        self.logic_to_symbol(value),
                        self.to_var_identifier(modification.component_idx, pin_idx)
                    ))
                }

                _ => unimplemented!()
            }
        }
    }

    pub fn contents(&self) -> String {
        self.lines.join("\n")
    }
}
