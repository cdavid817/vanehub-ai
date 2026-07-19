mod application_adapters;
mod credential_adapter;
mod runtime_bridge;
mod runtime_manager;
mod schema;
mod session_completion;
mod sqlite_repository;
mod transport_adapter;
pub(crate) mod transports;
mod wechat_authorization;

pub(crate) use application_adapters::{
    CommunicationsAgentExecutionAdapter, CommunicationsLoggingAdapter,
    CommunicationsOperationAdapter, CommunicationsSessionBindingAdapter, SystemCommunicationsClock,
};
pub(crate) use credential_adapter::CommunicationsCredentialAdapter;
pub(crate) use runtime_bridge::{BusyMessageProvider, CommunicationsInboundBridge};
pub(crate) use runtime_manager::ConnectorRuntimeManager;
pub(crate) use schema::{apply_schema, apply_session_source_schema};
pub(crate) use sqlite_repository::SqliteCommunicationsRepository;
pub(crate) use transport_adapter::CommunicationsTransportAdapter;
pub(crate) use wechat_authorization::WeChatAuthorizationService;
