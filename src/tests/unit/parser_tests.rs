use crate::script_engine::Object::*;
use crate::parser::*;
use crate::parser::Node::*;
use crate::logic::*;

#[test]
fn it_parses_integers() {
    assert_eq!(integer().parse(b"328").unwrap(), Constant(Integer(328)));
    assert_eq!(integer().parse(b"0").unwrap(), Constant(Integer(0)));
    assert_eq!(integer().parse(b"-123").unwrap(), Constant(Integer(-123)));
}

#[test]
fn it_parses_pin_definitions() {
    assert_eq!(pin_definition().parse(b"pin a;").unwrap(), PinDefinition("a".into()));
}

#[test]
fn it_parses_component_definitions() {
    assert_eq!(component_definition().parse(b"define component Something {
        pin a;
        pin b;

        script {
            a <- H;
            sleep(1000);
            a <- L;
            sleep(1000);
        }
    }
    ").unwrap(), ComponentDefinition {
        name: "Something".into(),
        body: Box::new(Body(vec![
            PinDefinition("a".into()),
            PinDefinition("b".into()),
            ScriptDefinition(Box::new(Body(vec![
                PinAssignment {
                    target: Box::new(Identifier("a".into())),
                    value: Box::new(Constant(LogicValue(Value::High)))
                },
                Sleep(Box::new(Constant(Integer(1000)))),
                PinAssignment {
                    target: Box::new(Identifier("a".into())),
                    value: Box::new(Constant(LogicValue(Value::Low)))
                },
                Sleep(Box::new(Constant(Integer(1000)))),
            ])))
        ])),

    })
}

#[test]
fn it_parses_connections() {
    assert_eq!(
        connect_definition().parse(b"connect(a.b, c.d);").unwrap(),
        Connect(vec![
            Accessor {
                target: Box::new(Identifier("a".into())),
                name: Box::new(Identifier("b".into())),
            },
            Accessor {
                target: Box::new(Identifier("c".into())),
                name: Box::new(Identifier("d".into())),
            },
        ]),
    );
}
