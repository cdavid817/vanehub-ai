mod application_adapters;
mod credential_adapter;
#[cfg(feature = "desktop-e2e")]
mod feishu_fixture;
mod lifecycle_events;
mod runtime_bridge;
mod runtime_manager;
mod schema;
mod sqlite_repository;
mod transport_adapter;
pub(crate) mod transports;
mod wechat_authorization;

pub(crate) use application_adapters::{
    CommunicationsAgentExecutionAdapter, CommunicationsLoggingAdapter,
    CommunicationsOperationAdapter, CommunicationsSessionBindingAdapter, SystemCommunicationsClock,
};
pub(crate) use credential_adapter::CommunicationsCredentialAdapter;
#[cfg(feature = "desktop-e2e")]
pub(crate) use feishu_fixture::{
    FeishuDesktopFixture, FeishuFixtureError, FeishuFixtureEvent, FeishuFixtureLedgerEntry,
    FeishuFixtureSetupResult,
};
pub(crate) use lifecycle_events::TauriConnectorLifecycleEvents;
pub(crate) use runtime_bridge::{BusyMessageProvider, CommunicationsInboundBridge};
pub(crate) use runtime_manager::ConnectorRuntimeManager;
pub(crate) use schema::{
    apply_schema, apply_session_binding_schema, apply_session_connector_access_schema,
    apply_session_source_schema,
};
pub(crate) use sqlite_repository::SqliteCommunicationsRepository;
pub(crate) use transport_adapter::{
    maintain_wechat_reply_contexts, CommunicationsTransportAdapter,
};
pub(crate) use wechat_authorization::WeChatAuthorizationService;
