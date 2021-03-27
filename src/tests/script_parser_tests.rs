use crate::script_parser::*;
use crate::script_engine::Object::*;
use crate::script_parser::Node::*;

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
    }").unwrap(), ComponentDefinition {
        name: "Something".into(),
        body: Box::new(
            Body(vec![PinDefinition("a".into()), PinDefinition("b".into())])
        )
    })
}
