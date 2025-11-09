use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::state::FlowContext;

pub type ConditionFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
pub type TransitionCondition = Arc<dyn Fn(&FlowContext) -> ConditionFuture<'_> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct FlowNode {
    pub name: String,
    pub kind: FlowNodeKind,
}

#[derive(Clone, Debug)]
pub enum FlowNodeKind {
    Agent(String),
    Terminal,
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
}

pub struct FlowBuilder {
    name: String,
    start: Option<String>,
    nodes: HashMap<String, FlowNode>,
    transitions: HashMap<String, Vec<FlowTransition>>,
}

impl FlowBuilder {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            start: None,
            nodes: HashMap::new(),
            transitions: HashMap::new(),
        }
    }

    pub fn add_agent_node(&mut self, name: &str, agent_name: &str) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            FlowNode {
                name: name.to_string(),
                kind: FlowNodeKind::Agent(agent_name.to_string()),
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
            },
        );
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
