use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    AgentMessage, DecisionBranch, DecisionPolicy, FlowBuilder, FlowContext, FlowNodeKind,
    FlowParameter, FlowParameterKind, FlowScopeKind, FlowVariable, JoinStrategy, condition_always,
    loop_condition_always, loop_condition_from_fn,
};

#[test]
fn decision_node_builder_supports_first_match_policy() {
    let mut builder = FlowBuilder::new("decision_flow");
    let branches = vec![
        DecisionBranch {
            name: Some("yes".to_string()),
            condition: Some(condition_always()),
            target: "positive".to_string(),
        },
        DecisionBranch {
            name: Some("fallback".to_string()),
            condition: None,
            target: "negative".to_string(),
        },
    ];

    builder
        .add_agent_node("positive", "positive_agent")
        .add_agent_node("negative", "negative_agent")
        .add_decision_node("decision", DecisionPolicy::FirstMatch, branches)
        .set_start("decision");

    let flow = builder.build();
    let decision_node = flow.node("decision").expect("decision node should exist");

    match &decision_node.kind {
        FlowNodeKind::Decision(decision) => {
            assert!(matches!(decision.policy, DecisionPolicy::FirstMatch));
            assert_eq!(decision.branches.len(), 2);
            assert_eq!(
                decision.branches[0].name.as_deref(),
                Some("yes"),
                "first branch should retain configured name"
            );
            assert!(
                decision.branches[0].condition.is_some(),
                "branch condition should be preserved"
            );
            assert_eq!(
                decision.branches[1].target, "negative",
                "fallback branch should route to negative target"
            );
        }
        other => panic!("expected decision node, got {:?}", other),
    }
}

#[test]
fn join_node_builder_tracks_inbound_sources() {
    let mut builder = FlowBuilder::new("join_flow");
    builder
        .add_agent_node("worker_a", "worker_a_agent")
        .add_agent_node("worker_b", "worker_b_agent")
        .add_join_node(
            "join",
            JoinStrategy::All,
            vec!["worker_a".to_string(), "worker_b".to_string()],
        )
        .set_start("worker_a")
        .connect("worker_a", "join")
        .connect("worker_b", "join");

    let flow = builder.build();
    let join_node = flow.node("join").expect("join node should exist");

    match &join_node.kind {
        FlowNodeKind::Join(join) => {
            assert!(matches!(join.strategy, JoinStrategy::All));
            assert_eq!(
                join.inbound.len(),
                2,
                "join node should preserve inbound sources"
            );
            assert!(
                join.inbound.contains(&"worker_a".to_string())
                    && join.inbound.contains(&"worker_b".to_string()),
                "join inbound set should include both workers"
            );
        }
        other => panic!("expected join node, got {:?}", other),
    }
}

#[tokio::test]
async fn loop_node_builder_and_helpers_behave() {
    let condition = loop_condition_from_fn(|ctx: &FlowContext| ctx.history().len() < 2);

    let mut builder = FlowBuilder::new("loop_flow");
    builder
        .add_loop_node(
            "loop",
            "body",
            Some(condition.clone()),
            Some(5),
            Some("exit".to_string()),
        )
        .add_agent_node("body", "loop_worker")
        .add_terminal_node("exit")
        .connect_loop("loop", "body", Some("exit"))
        .set_start("loop");

    let flow = builder.build();
    let loop_node = flow.node("loop").expect("loop node should exist");

    match &loop_node.kind {
        FlowNodeKind::Loop(loop_kind) => {
            assert_eq!(loop_kind.entry, "body");
            assert_eq!(loop_kind.max_iterations, Some(5));
            assert_eq!(loop_kind.exit.as_deref(), Some("exit"));
            assert!(
                loop_kind.condition.is_some(),
                "loop node should retain continuation condition"
            );
        }
        other => panic!("expected loop node, got {:?}", other),
    }

    let transitions = flow.transitions("loop");
    assert_eq!(
        transitions.len(),
        2,
        "loop transitions should include body edge and exit edge"
    );
    let has_exit_edge = transitions.iter().any(|transition| {
        transition.to == "exit" && transition.name.as_deref() == Some("loop_exit")
    });
    assert!(
        has_exit_edge,
        "loop node should expose explicit exit transition"
    );

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = FlowContext::new(ctx_store);

    // `loop_condition_always` should always evaluate to true.
    let always = loop_condition_always();
    assert!(
        (always)(&ctx).await,
        "loop_condition_always should return true"
    );

    // The custom condition allows the loop to continue while history length < 2.
    assert!(
        (condition)(&ctx).await,
        "loop condition should pass before history grows"
    );
    ctx.push_message(AgentMessage::system("first"));
    assert!(
        (condition)(&ctx).await,
        "loop condition should still pass with a single message"
    );
    ctx.push_message(AgentMessage::system("second"));
    assert!(
        !(condition)(&ctx).await,
        "loop condition should stop once history reaches the threshold"
    );
}

#[test]
fn flow_builder_collects_parameters_and_variables() {
    let mut builder = FlowBuilder::new("params_flow");
    builder
        .with_parameter(
            FlowParameter::input::<String>("request").with_description("incoming payload"),
        )
        .with_parameter(
            FlowParameter::output::<AgentMessage>("response").with_description("final response"),
        )
        .declare_variable(
            FlowVariable::new("retry_count", FlowScopeKind::Node("worker".to_string()))
                .with_default("0"),
        )
        .declare_variable(
            FlowVariable::new("correlation_id", FlowScopeKind::Global)
                .with_description("trace identifier"),
        )
        .add_terminal_node("finish")
        .set_start("finish");

    let flow = builder.build();
    let parameters = flow.parameters();
    assert_eq!(
        parameters.len(),
        2,
        "flow should record two declared parameters"
    );
    assert_eq!(parameters[0].name, "request");
    assert!(matches!(parameters[0].kind, FlowParameterKind::Input));
    assert_eq!(
        parameters[0].type_name.as_deref(),
        Some("alloc::string::String"),
        "type name should default to rust type path"
    );
    assert_eq!(parameters[1].name, "response");
    assert!(matches!(parameters[1].kind, FlowParameterKind::Output));

    let variables = flow.variables();
    assert_eq!(variables.len(), 2);
    assert_eq!(variables[0].name, "retry_count");
    assert_eq!(variables[0].default.as_deref(), Some("0"));
    assert!(matches!(
        variables[0].scope,
        FlowScopeKind::Node(ref node) if node == "worker"
    ));
    assert!(matches!(variables[1].scope, FlowScopeKind::Global));
    assert_eq!(
        variables[1].description.as_deref(),
        Some("trace identifier")
    );
}
