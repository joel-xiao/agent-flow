use crate::state::FlowScopeKind;
use std::collections::HashMap;

/// Flow 核心类型定义

/// Flow 工作流
#[derive(Clone)]
pub struct Flow {
    pub name: String,
    pub start: String,
    pub nodes: HashMap<String, crate::flow::nodes::FlowNode>,
    pub transitions: HashMap<String, Vec<FlowTransition>>,
    pub parameters: Vec<FlowParameter>,
    pub variables: Vec<FlowVariable>,
}

impl Flow {
    pub fn node(&self, name: &str) -> Option<&crate::flow::nodes::FlowNode> {
        self.nodes.get(name)
    }

    pub fn transitions(&self, name: &str) -> &[FlowTransition] {
        self.transitions
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn parameters(&self) -> &[FlowParameter] {
        &self.parameters
    }

    pub fn variables(&self) -> &[FlowVariable] {
        &self.variables
    }
}

/// Flow 转换
#[derive(Clone)]
pub struct FlowTransition {
    pub to: String,
    pub condition: Option<crate::flow::conditions::TransitionCondition>,
    pub name: Option<String>,
}

/// Flow 参数类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowParameterKind {
    Input,
    Output,
    InOut,
}

/// Flow 参数
#[derive(Clone, Debug)]
pub struct FlowParameter {
    pub name: String,
    pub kind: FlowParameterKind,
    pub type_name: Option<String>,
    pub description: Option<String>,
}

impl FlowParameter {
    pub fn new(name: impl Into<String>, kind: FlowParameterKind) -> Self {
        Self {
            name: name.into(),
            kind,
            type_name: None,
            description: None,
        }
    }

    pub fn input<T: 'static>(name: impl Into<String>) -> Self {
        let mut param = Self::new(name, FlowParameterKind::Input);
        param.type_name = Some(std::any::type_name::<T>().to_string());
        param
    }

    pub fn output<T: 'static>(name: impl Into<String>) -> Self {
        let mut param = Self::new(name, FlowParameterKind::Output);
        param.type_name = Some(std::any::type_name::<T>().to_string());
        param
    }

    pub fn in_out<T: 'static>(name: impl Into<String>) -> Self {
        let mut param = Self::new(name, FlowParameterKind::InOut);
        param.type_name = Some(std::any::type_name::<T>().to_string());
        param
    }

    pub fn with_type(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Flow 变量
#[derive(Clone, Debug)]
pub struct FlowVariable {
    pub name: String,
    pub scope: FlowScopeKind,
    pub default: Option<String>,
    pub description: Option<String>,
}

impl FlowVariable {
    pub fn new(name: impl Into<String>, scope: FlowScopeKind) -> Self {
        Self {
            name: name.into(),
            scope,
            default: None,
            description: None,
        }
    }

    pub fn with_default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

// FlowNode 在 nodes 模块中定义
