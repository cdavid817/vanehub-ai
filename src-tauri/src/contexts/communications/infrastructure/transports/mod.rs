pub mod dingtalk;
pub mod dingtalk_raw;
pub mod feishu;
pub mod feishu_raw;
pub mod http;
mod protocol;
mod runtime;
pub mod telegram;
mod token_cache;
pub mod wechat;
pub mod wecom;
pub mod wecom_raw;

#[cfg(feature = "desktop-e2e")]
pub(crate) use protocol::normalize_fixture;
#[cfg(test)]
pub(crate) use runtime::submit_inbound;
pub(crate) use runtime::SafeDiagnosticSink;
pub use runtime::{ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
