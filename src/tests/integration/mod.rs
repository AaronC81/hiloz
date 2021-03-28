use super::utils::create_model;
use crate::model::StepResult;
use crate::logic::Value;

// TODO: add a "_dump" function which will collect all dumped values into a
// vec, which we can then access through the model later on. This could make
// writing complex tests easier as we get the "true" mid-execution values as far
// as scripts are concerned

// Maybe have it be per-component in the CIS, like Vec<{ component_idx: usize, value: Object }>,
// then we collect them after each step into some model-maintained vec

#[test]
fn empty_model() {
    let mut model = create_model("");
    assert!(matches!(model.step(), StepResult::Halt));
}

#[test]
fn simple_model() {
    let mut model = create_model("
        define component ConstantHigh {
            pin out;

            script {
                out <- H;
            }
        }

        component h = ConstantHigh();
    ");
    assert_eq!(model.components[0].pins[0].value, Value::Unknown);
    assert!(matches!(model.step(), StepResult::Ok(_)));
    assert_eq!(model.components[0].pins[0].value, Value::High);
}

#[test]
fn simple_model_with_connection() {
    let mut model = create_model("
        define component ConstantHigh {
            pin out;

            script {
                out <- H;
            }
        }

        define component Stub {
            pin in;
        }

        component h = ConstantHigh();
        component s = Stub();

        connect(h.out, s.in);
    ");
}
