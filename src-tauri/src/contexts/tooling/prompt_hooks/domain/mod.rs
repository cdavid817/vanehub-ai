mod binding;
mod catalog;
mod classification;
mod error;
mod identity;
mod mutation;
mod ordering;
mod template;

pub(crate) use binding::{ManagedCliAgentId, PromptHookBindings};
pub(crate) use catalog::{builtin_prompt_hooks, BuiltinPromptHookDefinition};
pub(crate) use classification::{PromptHookCategory, PromptHookSource, PromptHookStage};
pub(crate) use error::PromptHookDomainError;
pub(crate) use identity::{PromptHookId, PromptHookName};
pub(crate) use mutation::{
    ensure_content_editable, ensure_deletable, ensure_enablement, ensure_identity_unchanged,
    PromptHookManifest,
};
pub(crate) use ordering::{
    compare_prompt_hook_order, ensure_order_available, PromptHookOrder, PromptHookOrderSlot,
};
pub(crate) use template::{
    PromptHookTemplate, PromptHookTemplateContext, PromptHookVariableDefinition,
    PROMPT_HOOK_VARIABLES,
};
