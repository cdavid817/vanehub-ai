mod category;
mod chat_configuration;
mod error;
pub(crate) mod evidence;
mod identity;
mod message;
pub(crate) mod recovery;
pub(crate) mod recovery_decision;
mod session;
mod session_seat;
mod usage_accounting;

pub(crate) use category::{CategoryName, SessionCategory};
pub(crate) use chat_configuration::{
    default_model_for_agent, model_id_from_cli, normalize_chat_preferences, normalize_reasoning,
    provider_for_agent, restore_chat_preferences, ChatConfigurationRequest, ChatPreferences,
};
pub(crate) use error::{ArchivedSessionAction, SessionsDomainError};
pub(crate) use identity::{CategoryId, MessageId, SessionId};
pub(crate) use message::{
    FileReference, FileReferenceSet, MessageRole, MessageStatus, SessionMessage,
};
pub(crate) use recovery::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger,
    SessionRecoveryReport, SessionRecoveryStatus,
};
pub(crate) use session::{
    LoopSessionRole, SessionActivation, SessionAggregate, SessionLifecycle, SessionOwner,
    SessionTitle,
};
pub(crate) use session_seat::{decode_seats, encode_seats, SessionSeat, SessionSeatRoleSnapshot};
pub(crate) use usage_accounting::{
    reconcile_cumulative_usage, AccountingUnit, CumulativeReconciliation, MeasurementKind,
    MeasurementQuality, TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose,
    UsageStatus,
};
