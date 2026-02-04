use crate::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum TheCodeNodeCallResult {
    Continue,
    Break,
}

pub type TheCodeNodeCall = fn(
    stack: &mut Vec<TheValue>,
    values: &mut TheCodeNodeData,
    sandbox: &mut TheCodeSandbox,
) -> TheCodeNodeCallResult;

#[derive(Clone, Debug)]
pub struct TheCodeNodeData {
    pub values: Vec<TheValue>,
    pub sub_functions: Vec<TheCodeFunction>,
    pub location: (u16, u16),
}

impl TheCodeNodeData {
    pub fn empty() -> Self {
        Self {
            values: vec![],
            sub_functions: vec![],
            location: (u16::MAX, u16::MAX),
        }
    }

    pub fn location(location: (u16, u16)) -> Self {
        Self {
            values: vec![],
            sub_functions: vec![],
            location,
        }
    }

    pub fn location_values(location: (u16, u16), values: Vec<TheValue>) -> Self {
        Self {
            values,
            sub_functions: vec![],
            location,
        }
    }

    pub fn values(values: Vec<TheValue>) -> Self {
        Self {
            values,
            sub_functions: vec![],
            location: (u16::MAX, u16::MAX),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TheCodeNode {
    pub call: TheCodeNodeCall,
    pub data: TheCodeNodeData,
}

impl TheCodeNode {
    pub fn new(call: TheCodeNodeCall, data: TheCodeNodeData) -> Self {
        Self { call, data }
    }
}
