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

#[test]
fn simple_model_with_locals_in_script() {
    let mut model = create_model("
        define component Component {
            script {
                var a;
                _dump(a);

                var b = 3;
                _dump(b);

                a = 10;
                _dump(a);

                b = H;
                b = L;
                _dump(b);
            }
        }

        component c = Component();
    ");
    model.run(100000, |_, _| {});
    assert_eq!(
        model.components[model.component_idx(&"c".to_string()).unwrap()].dumps,
        vec![
            se::Object::Null,
            se::Object::Integer(3),
            se::Object::Integer(10),
            se::Object::LogicValue(Value::Low),
        ]
    )
}

#[test]
fn simple_model_with_arithmetic() {
    let mut model = create_model("
        define component Component {
            script {
                var a = 5;
                _dump(a + -1);

                _dump(2 * 5 - 2 * 3 - (6 / 3));
            }
        }

        component c = Component();
    ");
    model.run(100000, |_, _| {});
    assert_eq!(
        model.components[model.component_idx(&"c".to_string()).unwrap()].dumps,
        vec![
            se::Object::Integer(4),
            se::Object::Integer(2),
        ]
    )
}

#[test]
fn simple_model_with_flow_constructs() {
    let mut model = create_model("
        define component Component {
            script {
                var i = 0;

                loop {
                    _dump(i);

                    if (i == 9) {
                        break;
                    }

                    i = i + 1;
                }
            }
        }

        component c = Component();
    ");
    model.run(100000, |_, _| {});
    assert_eq!(
        model.components[model.component_idx(&"c".to_string()).unwrap()].dumps,
        vec![
            se::Object::Integer(0),
            se::Object::Integer(1),
            se::Object::Integer(2),
            se::Object::Integer(3),
            se::Object::Integer(4),
            se::Object::Integer(5),
            se::Object::Integer(6),
            se::Object::Integer(7),
            se::Object::Integer(8),
            se::Object::Integer(9),
        ]
    )
}

#[test]
fn simple_model_with_logic() {
    let mut model = create_model("
        define component Component {
            script {
                _dump(H && H);
                _dump(H && L);
                _dump(L && L);

                _dump(H || L);
                _dump(L || L);

                _dump((L || H) && (H && H));
            }
        }

        component c = Component();
    ");
    model.run(100000, |_, _| {});
    assert_eq!(
        model.components[model.component_idx(&"c".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::Low),

            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            
            se::Object::LogicValue(Value::High),
        ]
    )
}

#[test]
fn simple_model_with_triggers() {
    let mut model = create_model("
        define component Component {
            pin out;
            script {
                out <- L;
                loop {
                    sleep(100);
                    out <- !out;
                    _dump(out);
                }
            }
        }

        define component NotGate {
            pin in;
            pin out;
            script {
                loop {
                    trigger;
                    _dump(in);
                    out <- !in;
                }
            }
        }

        component c = Component();
        component n = NotGate();

        connect(c.out, n.in);
        connect(n.out);
    ");
    model.run(550, |_, _| {});

    assert_eq!(
        model.components[model.component_idx(&"c".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::High),            
            se::Object::LogicValue(Value::Low),
        ]
    );

    assert_eq!(
        model.components[model.component_idx(&"n".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::High),
            se::Object::LogicValue(Value::Low),
            se::Object::LogicValue(Value::High),            
        ]
    );
}

#[test]
fn simple_model_with_constructor() {
    let mut model = create_model("
        define component Component {
            pin out;
            
            constructor(initial_value) {
                out <- initial_value;
            }

            script {
                _dump(out);
            }
        }

        component ch = Component(H);
        component cl = Component(L);
    ");

    model.construct();
    model.run(10, |_, _| {});

    assert_eq!(
        model.components[model.component_idx(&"ch".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::High),
        ]
    );

    assert_eq!(
        model.components[model.component_idx(&"cl".to_string()).unwrap()].dumps,
        vec![
            se::Object::LogicValue(Value::Low),
        ]
    );
}

#[test]
fn simple_model_with_component_variables() {
    let mut model = create_model("
        define component Component {
            var x;
            var y;
            
            constructor(cx, cy) {
                x = cx;
                y = cy;
            }

            script {
                _dump(x);
                _dump(y);
            }
        }

        component a = Component(1, 2);
        component b = Component(3, 4);
    ");

    model.construct();
    model.run(10, |_, _| {});

    assert_eq!(
        model.components[model.component_idx(&"a".to_string()).unwrap()].dumps,
        vec![
            se::Object::Integer(1),
            se::Object::Integer(2),
        ]
    );

    assert_eq!(
        model.components[model.component_idx(&"b".to_string()).unwrap()].dumps,
        vec![
            se::Object::Integer(3),
            se::Object::Integer(4),
        ]
    );
}
