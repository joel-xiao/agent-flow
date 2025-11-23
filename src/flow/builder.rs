use std::collections::HashMap;
use serde_json::Value;
use crate::flow::types::{Flow, FlowTransition, FlowParameter, FlowVariable};
use crate::flow::nodes::{
    FlowNode, FlowNodeKind, DecisionNode, DecisionPolicy, DecisionBranch,
    JoinNode, JoinStrategy, LoopNode, ToolNode,
};
use crate::flow::conditions::{TransitionCondition, LoopContinuation};

/// Flow 构建器
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

