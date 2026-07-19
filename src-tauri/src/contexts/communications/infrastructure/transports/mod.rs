pub mod dingtalk;
pub mod dingtalk_raw;
pub mod feishu;
pub mod feishu_raw;
pub mod http;
mod protocol;
mod runtime;
pub mod telegram;
pub mod wechat;
pub mod wecom;
pub mod wecom_raw;

#[cfg(test)]
pub(crate) use runtime::submit_inbound;
pub use runtime::{ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
