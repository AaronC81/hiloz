use super::script_engine as se;
use super::logic;

use pest::{Parser, iterators::Pair};

use std::str::{FromStr, from_utf8};
use std::error::Error;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Node {
    Constant(se::Object),
    Identifier(String),
    PinAssignment { target: Box<Node>, value: Box<Node> },
    Accessor { target: Box<Node>, name: Box<Node> },
    Sleep(Box<Node>),
    Dump(Box<Node>),
    Return(Box<Node>),
    LocalVariableDefinition { name: String, value: Option<Box<Node>> },
    LocalVariableAssignment { name: String, value: Box<Node> },

    PinDefinition(String),
    VariableDefinition(String),
    ComponentDefinition { name: String, body: Box<Node> },
    ConstructorDefinition { parameters: Vec<String>, body: Box<Node> },
    FunctionDefinition { name: String, parameters: Vec<String>, body: Box<Node> },
    ScriptDefinition(Box<Node>),

    ComponentInstantiation { instance_name: String, component_name: String, arguments: Vec<Node> },
    Connect(Vec<Node>),
    Pull { component: Vec<Node>, pull: logic::Value },

    LogicAnd(Box<Node>, Box<Node>),
    LogicOr(Box<Node>, Box<Node>),
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Equal(Box<Node>, Box<Node>),

    Loop(Box<Node>),
    If { condition: Box<Node>, body: Box<Node> },
    Break,

    Body(Vec<Node>),
    NodeList(Vec<Node>),

    EndOfInput,
}

use Node::*;
use se::Object::{*, self};

#[derive(Parser)]
#[grammar="model.pest"]
struct ModelParser;

impl ModelParser {
    fn pest_to_node(pest: Pair<Rule>) -> Result<Node, Box<dyn Error>> {
        match pest.as_rule() {
            Rule::integer =>
                Ok(Constant(Integer(i64::from_str(pest.as_str())?))),
            Rule::identifier =>
                Ok(Identifier(pest.as_str().into())),
            Rule::logic_value =>
                Ok(Constant(LogicValue(match pest.as_str() {
                    "H" => logic::Value::High,
                    "L" => logic::Value::Low,
                    "X" => logic::Value::Unknown,
                    _ => unreachable!(),
                }))),

            Rule::accessor => {
                let mut inner = pest.into_inner();
                let target = Self::pest_to_node(inner.next().unwrap())?;
                let name = Self::pest_to_node(inner.next().unwrap())?;

                Ok(Accessor {
                    target: Box::new(target),
                    name: Box::new(name),
                })
            },

            Rule::pin_definition =>
                Ok(PinDefinition(pest.into_inner().next().unwrap().as_str().into())),
            Rule::connect_definition => {
                let node_list = Self::pest_to_node(pest.into_inner().next().unwrap())?;
                
                if let NodeList(nodes) = node_list {
                    Ok(Connect(nodes))
                } else {
                    unreachable!();
                }
            },
            Rule::script_definition =>
                Ok(ScriptDefinition(Box::new(
                    Self::pest_to_node(pest.into_inner().next().unwrap())?
                ))),
            Rule::component_definition => {
                let mut inner = pest.into_inner();
                let name = inner.next().unwrap().as_str();
                let mut body = vec![];
                while let Some(node) = inner.next() {
                    body.push(Self::pest_to_node(node)?);
                }
                Ok(ComponentDefinition {
                    name: name.into(),
                    body: Box::new(Body(body)),
                })
            },
            Rule::component_instantiation => {
                let mut inner = pest.into_inner();
                let instance_name = inner.next().unwrap().as_str().into();
                let component_name = inner.next().unwrap().as_str().into();
                Ok(ComponentInstantiation {
                    instance_name,
                    component_name,
                    arguments: vec![],
                })
            }

            Rule::argument_list => {
                let mut inner = pest.into_inner();
                let mut nodes = vec![];

                while let Some(head_pair) = inner.next() {
                    nodes.push(Self::pest_to_node(head_pair)?);

                    // Set inner to the iterator for the child argument_list, if there is one
                    if let Some(pair) = inner.next() {
                        inner = pair.into_inner();
                    } else {
                        break;
                    }
                }

                Ok(NodeList(nodes))
            },

            Rule::statement =>
                Self::pest_to_node(pest.into_inner().next().unwrap()),
            Rule::pin_assignment => {
                let mut inner = pest.into_inner();
                let target = Self::pest_to_node(inner.next().unwrap())?;
                let value = Self::pest_to_node(inner.next().unwrap())?;
                Ok(PinAssignment {
                    target: Box::new(target),
                    value: Box::new(value),
                })
            },
            Rule::break_statement =>
                Ok(Break),
            Rule::sleep_statement =>
                Ok(Sleep(Box::new(
                    Self::pest_to_node(pest.into_inner().next().unwrap())?
                ))),
            Rule::dump_statement =>
                Ok(Dump(Box::new(
                    Self::pest_to_node(pest.into_inner().next().unwrap())?
                ))),
            Rule::local_variable_definition_statement => {
                let mut inner = pest.into_inner();
                let name = inner.next().unwrap().as_str();
                let value = if let Some(pair) = inner.next() {
                    Some(Box::new(Self::pest_to_node(pair)?))
                } else {
                    None
                };
                Ok(LocalVariableDefinition {
                    name: name.into(),
                    value,
                })
            },
            Rule::local_variable_assignment_statement => {
                let mut inner = pest.into_inner();
                let name = inner.next().unwrap().as_str();
                let value = Self::pest_to_node(inner.next().unwrap())?;
                Ok(LocalVariableAssignment {
                    name: name.into(),
                    value: Box::new(value),  
                })
            },
            Rule::statement_block | Rule::top =>
                Ok(Body(
                    pest.into_inner()
                        .map(|x| Self::pest_to_node(x))
                        .collect::<Result<Vec<_>, _>>()?
                )),

            Rule::expression =>
                Self::pest_to_node(pest.into_inner().next().unwrap()),

            Rule::binop_addsub | Rule::binop_muldiv | Rule::binop_eq | Rule::binop_andor => {
                let mut inner = pest.into_inner();
                let mut result = Self::pest_to_node(inner.next().unwrap())?;

                while let Some(operator) = inner.next() {
                    let operand = Box::new(Self::pest_to_node(inner.next().unwrap())?);

                    result = match operator.as_rule() {
                        Rule::operator_and => LogicAnd(Box::new(result), operand),
                        Rule::operator_or => LogicOr(Box::new(result), operand),
                        Rule::operator_eq => Equal(Box::new(result), operand),
                        Rule::operator_add => Add(Box::new(result), operand),
                        Rule::operator_sub => Subtract(Box::new(result), operand),
                        Rule::operator_mul => Multiply(Box::new(result), operand),
                        Rule::operator_div => Divide(Box::new(result), operand),

                        x => unreachable!("{:?} is not a binop", x)
                    }
                }

                Ok(result)
            },

            Rule::loop_statement =>
                Ok(Loop(Box::new(
                    Self::pest_to_node(pest.into_inner().next().unwrap())?
                ))),
            Rule::if_statement => {
                let mut inner = pest.into_inner();
                let condition = Self::pest_to_node(inner.next().unwrap())?;
                let body = Self::pest_to_node(inner.next().unwrap())?;
                Ok(If {
                    condition: Box::new(condition),
                    body: Box::new(body),
                })
            }

            Rule::operator_add | Rule::operator_sub | Rule::operator_mul | Rule::operator_div =>
                unreachable!("raw operator should not be processed"),

            Rule::EOI => Ok(EndOfInput),

            _ => unreachable!("unexpected rule {:?}", pest.as_rule()),
        }
    }
}

pub fn parse_rule(input: &str, rule: Rule) -> Result<Node, Box<dyn Error>> {
    let mut pairs = ModelParser::parse(rule, input)?;
    ModelParser::pest_to_node(pairs.next().unwrap())
}

pub fn parse(model: &str) -> Result<Node, Box<dyn Error>> {
    Ok(Body(
        ModelParser::parse(Rule::top, model)?
            .map(|n| ModelParser::pest_to_node(n))
            .collect::<Result<Vec<_>, _>>()?
    ))
}