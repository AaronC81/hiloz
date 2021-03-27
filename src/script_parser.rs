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
    Assignment { target: Box<Node>, value: Box<Node> },
    Accessor { target: Box<Node>, name: Box<Node> },
    Sleep(Box<Node>),
    Return(Box<Node>),

    PinDefinition(String),
    VariableDefinition(String),
    ComponentDefinition { name: String, body: Box<Node> },
    ConstructorDefinition { parameters: Vec<String>, body: Box<Node> },
    FunctionDefinition { name: String, parameters: Vec<String>, body: Box<Node> },

    ComponentInstantiation { instance_name: Box<Node>, component_name: Box<Node>, arguments: Vec<Node> },
    Connect(Vec<Node>),
    Pull { component: Vec<Node>, pull: logic::Value },

    Body(Vec<Node>),
}

use Node::*;
use se::Object::*;

fn space() -> Parser<u8, ()> { is_a(multispace).repeat(1..).discard() }
fn semi() -> Parser<u8, ()> { (space() + sym(b';') + space()).discard() }
fn lbrace() -> Parser<u8, ()> { (space() + sym(b'{') + space()).discard() }
fn rbrace() -> Parser<u8, ()> { (space() + sym(b'}') + space()).discard() }

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
    // TODO: reject keywords
    ((is_a(alpha) | sym(b'_')) + (is_a(alpha) | is_a(digit) | sym(b'_')).repeat(1..))
        .collect()
        .convert(from_utf8)
        .map(Into::into)
}

pub fn identifier() -> Parser<u8, Node> {
    raw_identifier().map(|s| Identifier(s.into()))
}

pub fn pin_definition() -> Parser<u8, Node> {
    (seq(b"pin") + space() + raw_identifier() + semi())
        .map(|((_, id), _)| PinDefinition(id))
}

pub fn component_definition() -> Parser<u8, Node> {
    (
        seq(b"define") + space() + seq(b"component") + space()
        + raw_identifier() + space()
        + component_definition()
    ).map(|(((_, name), _,), body)| ComponentDefinition {
        name,
        body: Box::new(body),
    })
}

pub fn component_definition_body() -> Parser<u8, Node> {
    // TODO
    (lbrace() + rbrace()).map(|_| Node::Body(vec![]))
}
