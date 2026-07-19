mod binding;
mod catalog;
mod drift;
mod error;
mod identity;
mod metadata;
mod mount;
mod mutation;
mod source;

pub(crate) use binding::{plan_binding_change, plan_enablement, SkillBindingPlan};
pub(crate) use catalog::{builtin_definition, builtin_definitions};
pub(crate) use drift::{
    detect_drift, RegisteredSkillInspection, SkillBindingInspection, SkillDriftInspection,
    SkillDriftIssue, SkillDriftIssueType, SkillMountObservation, SkillSourceInspection,
    UnregisteredSkillInspection,
};
pub(crate) use error::SkillDomainError;
pub(crate) use identity::{SkillId, SkillKey, SkillLocation, SkillScope};
pub(crate) use metadata::SkillMetadata;
pub(crate) use mount::{default_mount_path, SkillMountPath};
pub(crate) use mutation::{
    builtin_restore_plan, deletion_policy, source_for_user_create, validate_create_identity,
    validate_update_identity,
};
pub(crate) use source::SkillSource;
