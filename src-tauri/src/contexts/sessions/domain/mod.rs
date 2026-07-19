mod category;
mod chat_configuration;
mod error;
mod identity;
mod message;
mod session;

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
pub(crate) use session::{
    SessionActivation, SessionAggregate, SessionLifecycle, SessionOwner, SessionTitle,
};
