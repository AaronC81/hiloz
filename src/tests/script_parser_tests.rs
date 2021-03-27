use crate::script_parser::*;
use crate::script_engine::Object::*;
use crate::script_parser::Node::*;

#[test]
fn it_parses_integers() {
    assert_eq!(integer().parse(b"328").unwrap(), Constant(Integer(328)));
    assert_eq!(integer().parse(b"0").unwrap(), Constant(Integer(0)));
    assert_eq!(integer().parse(b"-123").unwrap(), Constant(Integer(-123)));
}
