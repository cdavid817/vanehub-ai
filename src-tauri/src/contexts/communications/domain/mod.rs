mod authorization;
mod connector;
mod delivery;
mod error;
mod routing;
mod status;

pub(crate) use authorization::{
    AuthorizationAttempt, AuthorizationObservation, AuthorizationStatus,
};
pub(crate) use connector::{
    builtin_descriptors, connector_field_definitions, ConnectorConfig, ConnectorDescriptor,
    ConnectorFieldStorage, ConnectorKind,
};
pub(crate) use delivery::{
    classify_safe_code, pending_delivery_admission, safe_platform_status_code, split_text,
    ConnectorErrorClass, DeduplicationDecision, DeliveryAdmission, InboundDisposition,
    NormalizedInbound, OutboundText, MAX_PENDING_PER_CHAT,
};
pub(crate) use error::CommunicationsDomainError;
pub(crate) use routing::{
    ChatBinding, ChatBindingKey, CheckpointKey, ConnectorCheckpoint, InboundEventIdentity,
    RoutingSettings,
};
pub(crate) use status::{ConnectorHealth, ConnectorLifecycle, ConnectorStatus};
