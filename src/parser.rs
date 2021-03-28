use super::script_engine as se;
use super::logic;

use pom::parser::*;
use pom::char_class::*;
use pom::Parser;

use std::str::{FromStr, from_utf8};
use std::error::Error;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Node {
    Constant(se::Object),
    Identifier(String),
    PinAssignment { target: Box<Node>, value: Box<Node> },
    Accessor { target: Box<Node>, name: Box<Node> },
    Sleep(Box<Node>),
    Return(Box<Node>),

    PinDefinition(String),
    VariableDefinition(String),
    ComponentDefinition { name: String, body: Box<Node> },
    ConstructorDefinition { parameters: Vec<String>, body: Box<Node> },
    FunctionDefinition { name: String, parameters: Vec<String>, body: Box<Node> },
    ScriptDefinition(Box<Node>),

    ComponentInstantiation { instance_name: String, component_name: String, arguments: Vec<Node> },
    Connect(Vec<Node>),
    Pull { component: Vec<Node>, pull: logic::Value },

    Body(Vec<Node>),
}

use Node::*;
use se::Object::*;

fn space() -> Parser<u8, ()> { is_a(multispace).repeat(0..).discard() }
fn must_space() -> Parser<u8, ()> { is_a(multispace).repeat(1..).discard().name("whitespace") }
fn semi() -> Parser<u8, ()> { (space() + sym(b';') + space()).discard().name("semicolon") }
fn lbrace() -> Parser<u8, ()> { (space() + sym(b'{') + space()).discard().name("left brace") }
fn rbrace() -> Parser<u8, ()> { (space() + sym(b'}') + space()).discard().name("right brace") }
fn lparen() -> Parser<u8, ()> { (space() + sym(b'(') + space()).discard().name("left parenthesis") }
fn rparen() -> Parser<u8, ()> { (space() + sym(b')') + space()).discard().name("right parenthesis") }

pub fn raw_integer() -> Parser<u8, i64> {
    (sym(b'-').opt() + is_a(digit).repeat(1..))
        .collect()
        .convert(from_utf8)
        .convert(i64::from_str)
}

pub fn integer() -> Parser<u8, Node> {
    raw_integer().map(|s| Constant(Integer(s)))
}

pub fn raw_identifier() -> Parser<u8, String> {
    // TODO: reject keywords or logic value literals
    ((is_a(alpha) | sym(b'_')) + (is_a(alpha) | is_a(digit) | sym(b'_')).repeat(0..))
        .collect()
        .convert(from_utf8)
        .map(Into::into)
}

pub fn identifier() -> Parser<u8, Node> {
    raw_identifier().map(|s| Identifier(s.into()))
}

pub fn logic_value() -> Parser<u8, Node> {
    sym(b'H').map(|_| Constant(LogicValue(logic::Value::High)))
    | sym(b'L').map(|_| Constant(LogicValue(logic::Value::Low)))
    | sym(b'X').map(|_| Constant(LogicValue(logic::Value::Unknown)))
}

pub fn pin_definition() -> Parser<u8, Node> {
    (seq(b"pin") + must_space() + raw_identifier() + semi())
        .map(|((_, id), _)| PinDefinition(id))
}

pub fn script_definition() -> Parser<u8, Node> {
    (seq(b"script") + space() + script_block())
        .map(|(_, body)| ScriptDefinition(Box::new(body)))
}

pub fn component_definition() -> Parser<u8, Node> {
    (
        seq(b"define") + must_space() + seq(b"component") + must_space()
        + raw_identifier() + space()
        + component_definition_body()
    )
        .map(|(((_, name), _,), body)| ComponentDefinition {
            name,
            body: Box::new(body),
        })
}

pub fn component_definition_body() -> Parser<u8, Node> {
    (
        lbrace()
        + (pin_definition() | script_definition()).repeat(0..)
        + rbrace()
    )
        .map(|((_, defs), _)| Node::Body(defs))
}

pub fn script_block() -> Parser<u8, Node> {
    (
        lbrace()
        + space()
        + script_statement().repeat(0..)
        + space()
        + rbrace()
    )
        .map(|(((_, e), _), _)| Body(e))
}

pub fn script_statement() -> Parser<u8, Node> {
    ((script_sleep_statement() | pin_assignment() | script_expression())
        + space() + semi()).map(|((e, _), _)| e)
}

pub fn script_expression() -> Parser<u8, Node> {
    integer() | logic_value() | identifier()
}

pub fn pin_assignment() -> Parser<u8, Node> {
    (identifier() + space() + seq(b"<-") + space() + script_expression())
        .map(|((((i, _), _), _), e)| PinAssignment {
            target: Box::new(i),
            value: Box::new(e),
        })
}

pub fn script_sleep_statement() -> Parser<u8, Node> {
    (seq(b"sleep") + space() + lparen() + space() + script_expression() + space() + rparen())
        .map(|(((_, e), _), _)| Sleep(Box::new(e)))
}

pub fn component_instantiation() -> Parser<u8, Node> {
    (
        seq(b"component") + must_space() + raw_identifier()
        + space() + sym(b'=') + space() + raw_identifier() + space()
        // TODO: constructor parameters
        + lparen() + space() + rparen()
        + semi()
    // we lisp now
    ).map(|((((((((((_, i), _), _), _), c), _), _), _), _), _)| ComponentInstantiation {
        instance_name: i,
        component_name: c,
        arguments: vec![],
    })
}

pub fn top_level() -> Parser<u8, Node> {
    (space() + (component_definition() | component_instantiation()) + space())
        .map(|((_, c), _)| c)
        .repeat(0..)
        .map(|x| Body(x)) 
}
