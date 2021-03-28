use super::utils::create_model;
use crate::model::StepResult;
use crate::logic::Value;

#[test]
fn empty_model() {
    let mut model = create_model(b"");
    assert_eq!(model.step(), StepResult::Halt);
}

#[test]
fn simple_model() {
    let mut model = create_model(b"
        define component ConstantHigh {
            pin out;

            script {
                out <- H;
            }
        }

        component h = ConstantHigh();
    ");
    assert_eq!(model.components[0].pins[0].value, Value::Unknown);
    assert_eq!(model.step(), StepResult::Ok);
    assert_eq!(model.components[0].pins[0].value, Value::High);
}
