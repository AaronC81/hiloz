use crate::script_engine::Object::*;
use crate::parser::*;
use crate::parser::Node::*;
use crate::logic::*;

#[test]
fn it_parses_integers() {
    assert_eq!(parse_rule("328", Rule::integer).unwrap(), Constant(Integer(328)));
    assert_eq!(parse_rule("0", Rule::integer).unwrap(), Constant(Integer(0)));
    assert_eq!(parse_rule("-123", Rule::integer).unwrap(), Constant(Integer(-123)));
}

#[test]
fn it_parses_identifiers() {
    assert_eq!(parse_rule("a", Rule::identifier).unwrap(), Identifier("a".into()));
    assert_eq!(parse_rule("foo", Rule::identifier).unwrap(), Identifier("foo".into()));
}

#[test]
fn it_parses_pin_definitions() {
    assert_eq!(parse_rule("pin a;", Rule::pin_definition).unwrap(), PinDefinition("a".into()));
}

#[test]
fn it_parses_component_definitions() {
    assert_eq!(parse_rule("define component Something {
        pin a;
        pin b;

        script {
            a <- H;
            sleep(1000);
            a <- L;
            sleep(1000);
        }
    }
    ", Rule::component_definition).unwrap(), ComponentDefinition {
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
        parse_rule("connect(a.b, c.d);", Rule::connect_definition).unwrap(),
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

#[test]
fn it_parses_complex_expressions() {
    assert_eq!(
        parse_rule("a.b + (3 / 4 / 1 + 1 + a.c / 4) * (3 + 2) + 5", Rule::expression).unwrap(),
        Add(
            Box::new(Add(
                Box::new(Accessor {
                    target: Box::new(Identifier("a".into())),
                    name: Box::new(Identifier("b".into())),
                }),
                Box::new(Multiply(
                    Box::new(Add(
                        Box::new(Add(
                            Box::new(Divide(
                                Box::new(Divide(
                                    Box::new(Constant(Integer(3))),
                                    Box::new(Constant(Integer(4))),
                                )),
                                Box::new(Constant(Integer(1))),
                            )),
                            Box::new(Constant(Integer(1))),
                        )),
                        Box::new(Divide(
                            Box::new(Accessor {
                                target: Box::new(Identifier("a".into())),
                                name: Box::new(Identifier("c".into())),
                            }),
                            Box::new(Constant(Integer(4))),
                        )),
                    )),
                    Box::new(Add(
                        Box::new(Constant(Integer(3))),
                        Box::new(Constant(Integer(2))),
                    )),
                ))
            )),
            Box::new(Constant(Integer(5)))
        )
    )
}
