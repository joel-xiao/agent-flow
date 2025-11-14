use std::any::type_name;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::state::{FlowContext, FlowScopeKind};

pub mod loader;

pub type ConditionFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
pub type TransitionCondition = Arc<dyn Fn(&FlowContext) -> ConditionFuture<'_> + Send + Sync>;
pub type LoopContinuationFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
pub type LoopContinuation = Arc<dyn Fn(&FlowContext) -> LoopContinuationFuture<'_> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct FlowNode {
    pub name: String,
    pub kind: FlowNodeKind,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub enum FlowNodeKind {
    Agent(String),
    Terminal,
    Decision(DecisionNode),
    Join(JoinNode),
    Loop(LoopNode),
    Tool(ToolNode),
}

#[derive(Clone)]
pub struct DecisionNode {
    pub policy: DecisionPolicy,
    pub branches: Vec<DecisionBranch>,
}

#[derive(Clone, Debug)]
pub enum DecisionPolicy {
    FirstMatch,
    AllMatches,
}

#[derive(Clone)]
pub struct DecisionBranch {
    pub name: Option<String>,
    pub condition: Option<TransitionCondition>,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct JoinNode {
    pub strategy: JoinStrategy,
    pub inbound: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum JoinStrategy {
    All,
    Any,
    Count(usize),
}

#[derive(Clone)]
pub struct LoopNode {
    pub entry: String,
    pub condition: Option<LoopContinuation>,
    pub max_iterations: Option<u32>,
    pub exit: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolNode {
    pub pipeline: String,
}

#[derive(Clone)]
pub struct FlowTransition {
    pub to: String,
    pub condition: Option<TransitionCondition>,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct Flow {
    pub name: String,
    pub start: String,
    pub nodes: HashMap<String, FlowNode>,
    pub transitions: HashMap<String, Vec<FlowTransition>>,
    pub parameters: Vec<FlowParameter>,
    pub variables: Vec<FlowVariable>,
}

impl Flow {
    pub fn node(&self, name: &str) -> Option<&FlowNode> {
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

pub struct FlowBuilder {
    name: String,
    start: Option<String>,
    nodes: HashMap<String, FlowNode>,
    transitions: HashMap<String, Vec<FlowTransition>>,
    parameters: Vec<FlowParameter>,
    variables: Vec<FlowVariable>,
}

impl FlowBuilder {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            start: None,
            nodes: HashMap::new(),
            transitions: HashMap::new(),
            parameters: Vec::new(),
            variables: Vec::new(),
        }
    }

    pub fn add_agent_node(&mut self, name: &str, agent_name: &str) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Agent(agent_name.to_string()),
                metadata: None,
            },
        );
        self
    }

    pub fn add_terminal_node(&mut self, name: &str) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Terminal,
                metadata: None,
            },
        );
        self
    }

    pub fn add_decision_node(
        &mut self,
        name: &str,
        policy: DecisionPolicy,
        branches: Vec<DecisionBranch>,
    ) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Decision(DecisionNode { policy, branches }),
                metadata: None,
            },
        );
        self
    }

    pub fn add_join_node(
        &mut self,
        name: &str,
        strategy: JoinStrategy,
        inbound: Vec<String>,
    ) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Join(JoinNode { strategy, inbound }),
                metadata: None,
            },
        );
        self
    }

    pub fn add_loop_node(
        &mut self,
        name: &str,
        entry: &str,
        condition: Option<LoopContinuation>,
        max_iterations: Option<u32>,
        exit: Option<String>,
    ) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Loop(LoopNode {
                    entry: entry.to_string(),
                    condition,
                    max_iterations,
                    exit,
                }),
                metadata: None,
            },
        );
        self
    }

    pub fn set_node_metadata(&mut self, name: &str, metadata: Value) -> &mut Self {
        if let Some(node) = self.nodes.get_mut(name) {
            node.metadata = Some(metadata);
        }
        self
    }

    pub fn add_tool_node(&mut self, name: &str, pipeline: &str) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Tool(ToolNode {
                    pipeline: pipeline.to_string(),
                }),
                metadata: None,
            },
        );
        self
    }

    pub fn with_parameter(&mut self, parameter: FlowParameter) -> &mut Self {
        self.parameters.push(parameter);
        self
    }

    pub fn with_parameters<I>(&mut self, parameters: I) -> &mut Self
    where
        I: IntoIterator<Item = FlowParameter>,
    {
        self.parameters.extend(parameters);
        self
    }

    pub fn declare_variable(&mut self, variable: FlowVariable) -> &mut Self {
        self.variables.push(variable);
        self
    }

    pub fn set_start(&mut self, name: &str) -> &mut Self {
        self.start = Some(name.to_string());
        self
    }

    pub fn connect(&mut self, from: &str, to: &str) -> &mut Self {
        self.connect_named(from, to, None)
    }

    pub fn connect_named(&mut self, from: &str, to: &str, name: Option<String>) -> &mut Self {
        self.transitions
            .entry(from.to_string())
            .or_default()
            .push(FlowTransition {
                to: to.to_string(),
                condition: None,
                name,
            });
        self
    }

    pub fn connect_if(
        &mut self,
        from: &str,
        to: &str,
        condition: TransitionCondition,
    ) -> &mut Self {
        self.connect_conditional(from, to, condition)
    }

    pub fn connect_if_named(
        &mut self,
        from: &str,
        to: &str,
        name: Option<String>,
        condition: TransitionCondition,
    ) -> &mut Self {
        self.connect_conditional_named(from, to, name, condition)
    }

    pub fn connect_loop(&mut self, loop_node: &str, body: &str, exit: Option<&str>) -> &mut Self {
        self.connect(loop_node, body);
        self.connect(body, loop_node);
        if let Some(exit_target) = exit {
            self.transitions
                .entry(loop_node.to_string())
                .or_default()
                .push(FlowTransition {
                    to: exit_target.to_string(),
                    condition: None,
                    name: Some("loop_exit".to_string()),
                });
        }
        self
    }

    pub fn connect_conditional(
        &mut self,
        from: &str,
        to: &str,
        condition: TransitionCondition,
    ) -> &mut Self {
        self.connect_conditional_named(from, to, None, condition)
    }

    pub fn connect_conditional_named(
        &mut self,
        from: &str,
        to: &str,
        name: Option<String>,
        condition: TransitionCondition,
    ) -> &mut Self {
        self.transitions
            .entry(from.to_string())
            .or_default()
            .push(FlowTransition {
                to: to.to_string(),
                condition: Some(condition),
                name,
            });
        self
    }

    pub fn build(self) -> Flow {
        let start = self
            .start
            .or_else(|| self.nodes.keys().next().cloned())
            .unwrap_or_default();
        Flow {
            name: self.name,
            start,
            nodes: self.nodes,
            transitions: self.transitions,
            parameters: self.parameters,
            variables: self.variables,
        }
    }
}

pub fn condition_from_fn<F>(func: F) -> TransitionCondition
where
    F: Fn(&FlowContext) -> bool + Send + Sync + 'static,
{
    let func = Arc::new(func);
    Arc::new(move |ctx| {
        let func = Arc::clone(&func);
        Box::pin(async move { func(ctx) })
    })
}

pub fn condition_always() -> TransitionCondition {
    Arc::new(|_| Box::pin(async move { true }))
}

pub fn condition_state_equals<K, V>(key: K, expected: V) -> TransitionCondition
where
    K: Into<String> + Send + Sync + 'static,
    V: Into<String> + Send + Sync + 'static,
{
    let key = key.into();
    let expected = expected.into();
    Arc::new(move |ctx| {
        let store = ctx.store();
        let key = key.clone();
        let expected = expected.clone();
        Box::pin(async move {
            match store.get(&key).await {
                Ok(Some(value)) => value == expected,
                _ => false,
            }
        })
    })
}

pub fn condition_state_not_equals<K, V>(key: K, value: V) -> TransitionCondition
where
    K: Into<String> + Send + Sync + 'static,
    V: Into<String> + Send + Sync + 'static,
{
    let key = key.into();
    let value = value.into();
    Arc::new(move |ctx| {
        let store = ctx.store();
        let key = key.clone();
        let value = value.clone();
        Box::pin(async move {
            match store.get(&key).await {
                Ok(Some(current)) => current != value,
                Ok(None) => true,
                Err(_) => false,
            }
        })
    })
}

pub fn condition_state_exists<K>(key: K) -> TransitionCondition
where
    K: Into<String> + Send + Sync + 'static,
{
    let key = key.into();
    Arc::new(move |ctx| {
        let store = ctx.store();
        let key = key.clone();
        Box::pin(async move { store.get(&key).await.ok().flatten().is_some() })
    })
}

pub fn condition_state_absent<K>(key: K) -> TransitionCondition
where
    K: Into<String> + Send + Sync + 'static,
{
    let key = key.into();
    Arc::new(move |ctx| {
        let store = ctx.store();
        let key = key.clone();
        Box::pin(async move { store.get(&key).await.ok().flatten().is_none() })
    })
}

pub fn loop_condition_from_fn<F>(func: F) -> LoopContinuation
where
    F: Fn(&FlowContext) -> bool + Send + Sync + 'static,
{
    let func = Arc::new(func);
    Arc::new(move |ctx| {
        let func = Arc::clone(&func);
        Box::pin(async move { func(ctx) })
    })
}

pub fn loop_condition_always() -> LoopContinuation {
    Arc::new(|_| Box::pin(async move { true }))
}

impl fmt::Debug for DecisionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecisionNode")
            .field("policy", &self.policy)
            .field("branches", &self.branches)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowParameterKind {
    Input,
    Output,
    InOut,
}

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
        param.type_name = Some(type_name::<T>().to_string());
        param
    }

    pub fn output<T: 'static>(name: impl Into<String>) -> Self {
        let mut param = Self::new(name, FlowParameterKind::Output);
        param.type_name = Some(type_name::<T>().to_string());
        param
    }

    pub fn in_out<T: 'static>(name: impl Into<String>) -> Self {
        let mut param = Self::new(name, FlowParameterKind::InOut);
        param.type_name = Some(type_name::<T>().to_string());
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

impl fmt::Debug for DecisionBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecisionBranch")
            .field("name", &self.name)
            .field("target", &self.target)
            .field("has_condition", &self.condition.as_ref().map(|_| true))
            .finish()
    }
}

impl fmt::Debug for LoopNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoopNode")
            .field("entry", &self.entry)
            .field("max_iterations", &self.max_iterations)
            .field("exit", &self.exit)
            .field("has_condition", &self.condition.as_ref().map(|_| true))
            .finish()
    }
}

#[derive(Default)]
pub struct FlowRegistry {
    flows: HashMap<String, Flow>,
}

impl FlowRegistry {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
        }
    }

    pub fn register(&mut self, flow: Flow) {
        self.flows.insert(flow.name.clone(), flow);
    }

    pub fn get(&self, name: &str) -> Option<&Flow> {
        self.flows.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = &Flow> {
        self.flows.values()
    }
}
