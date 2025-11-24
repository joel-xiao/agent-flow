use crate::state::FlowContext;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Flow 条件类型定义

/// 条件 Future 类型
pub type ConditionFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

/// 转换条件类型
pub type TransitionCondition = Arc<dyn Fn(&FlowContext) -> ConditionFuture<'_> + Send + Sync>;

/// 循环继续 Future 类型
pub type LoopContinuationFuture<'a> = Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

/// 循环继续条件类型
pub type LoopContinuation = Arc<dyn Fn(&FlowContext) -> LoopContinuationFuture<'_> + Send + Sync>;

/// 从函数创建转换条件
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

/// 总是为真的条件
pub fn condition_always() -> TransitionCondition {
    Arc::new(|_| Box::pin(async move { true }))
}

/// 状态等于指定值的条件
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

/// 状态不等于指定值的条件
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

/// 状态存在的条件
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

/// 状态不存在的条件
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

/// 从函数创建循环继续条件
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

/// 总是继续的循环条件
pub fn loop_condition_always() -> LoopContinuation {
    Arc::new(|_| Box::pin(async move { true }))
}
