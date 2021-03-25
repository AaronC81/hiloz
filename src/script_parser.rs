use super::script_engine as se;
use super::logic;

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
