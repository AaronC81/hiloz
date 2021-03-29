use super::utils::create_model;
use crate::model::{ConnectedComponents, StepResult};
use crate::logic::Value;
use crate::script_engine as se;

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
                sleep(50);
                out <- H;
            }
        }

        define component Stub {
            pin in;

            script {
                _dump(in);
                sleep(100);
                _dump(in);
            }
        }

        component h = ConstantHigh();
        component s = Stub();

        connect(h.out, s.in);
    ");
    model.run(100000, |_, _| {});
    assert_eq!(
        model.components[model.component_idx(&"s".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::Unknown),
            se::Object::LogicValue(Value::High),
        ]
    )
}
