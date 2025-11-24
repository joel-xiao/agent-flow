use std::env;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 日志配置
pub struct LoggingConfig;

impl LoggingConfig {
    /// 初始化日志系统
    ///
    /// 支持通过环境变量配置：
    /// - RUST_LOG: 设置日志级别（error, warn, info, debug, trace）
    /// - AGENTFLOW_DEBUG: 启用详细调试输出
    ///
    /// 使用示例：
    /// ```no_run
    /// use agentflow::utils::LoggingConfig;
    ///
    /// fn main() {
    ///     LoggingConfig::init();
    ///     // 现在可以使用 tracing 宏
    /// }
    /// ```
    pub fn init() {
        let is_debug = env::var("AGENTFLOW_DEBUG").is_ok();

        let env_filter = match EnvFilter::try_from_default_env() {
            Ok(filter) => filter,
            Err(_) => {
                if is_debug {
                    EnvFilter::new("agentflow=debug,info")
                } else {
                    EnvFilter::new("agentflow=info,warn")
                }
            }
        };

        let fmt_layer = if is_debug {
            fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
        } else {
            fmt::layer()
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
        };

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        if is_debug {
            tracing::debug!("调试模式已启用");
        }
    }

    /// 初始化日志系统（带自定义过滤器）
    pub fn init_with_filter(filter: &str) {
        let env_filter = EnvFilter::new(filter);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer())
            .init();
    }

    /// 检查是否启用调试模式
    pub fn is_debug() -> bool {
        env::var("AGENTFLOW_DEBUG").is_ok()
    }
}

/// 便捷宏：记录带上下文的错误
#[macro_export]
macro_rules! log_error {
    ($err:expr) => {
        tracing::error!(error = ?$err, "错误发生")
    };
    ($err:expr, $($key:tt = $value:expr),+) => {
        tracing::error!(error = ?$err, $($key = $value),+)
    };
}

/// 便捷宏：记录带上下文的警告
#[macro_export]
macro_rules! log_warn {
    ($msg:expr) => {
        tracing::warn!($msg)
    };
    ($msg:expr, $($key:tt = $value:expr),+) => {
        tracing::warn!($msg, $($key = $value),+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_debug() {
        env::remove_var("AGENTFLOW_DEBUG");
        assert!(!LoggingConfig::is_debug());

        env::set_var("AGENTFLOW_DEBUG", "1");
        assert!(LoggingConfig::is_debug());

        env::remove_var("AGENTFLOW_DEBUG");
    }
}
