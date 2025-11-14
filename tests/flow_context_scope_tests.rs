use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{FlowContext, FlowScopeKind, FlowVariables};

#[tokio::test]
async fn session_context_persists_values() -> anyhow::Result<()> {
    let store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = FlowContext::new(Arc::clone(&store));
    let session = ctx.session();

    session.set("user_id", "alice").await?;
    session.set("request_id", "req-123").await?;

    assert_eq!(session.get("user_id").await?, Some("alice".to_string()));
    assert_eq!(
        store.get("session:user_id").await?,
        Some("alice".to_string())
    );

    session.delete("request_id").await?;
    assert_eq!(session.get("request_id").await?, None);
    Ok(())
}

#[tokio::test]
async fn scope_push_and_drop_clears_variables() -> anyhow::Result<()> {
    let store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = FlowContext::new(Arc::clone(&store));
    let variables: FlowVariables = ctx.variables();

    variables.set_global("flow_id", "demo-flow").await?;
    assert_eq!(
        variables.get("flow_id").await,
        Some("demo-flow".to_string())
    );

    let branch_value = {
        let node_scope = ctx.scope(FlowScopeKind::Node("worker".to_string()));
        node_scope.set("attempt", "1").await?;
        node_scope.get("attempt").await
    };
    assert_eq!(
        branch_value,
        Some("1".to_string()),
        "scope get should return value before guard drops"
    );

    assert!(
        variables.get("attempt").await.is_none(),
        "scope variables should be cleared after guard drop"
    );
    assert_eq!(
        variables.get("flow_id").await,
        Some("demo-flow".to_string()),
        "global variables should remain after scope drop"
    );
    Ok(())
}
