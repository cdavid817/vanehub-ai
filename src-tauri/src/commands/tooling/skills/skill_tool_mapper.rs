use super::skill_tool_dto::{SkillToolDiagnosticDto, SkillToolRevisionDto};
use crate::contexts::tooling::skill_tools::api::SkillToolRevisionState;
use crate::contexts::tooling::skill_tools::api::{
    SkillToolApplicationError, SkillToolOwnerId, SkillToolPackageRef, SkillToolRevision,
    SkillToolScope, SkillToolSourceScope,
};

pub(crate) fn owner(
    skill_id: &str,
    scope: &str,
    workspace_path: Option<&str>,
) -> Result<SkillToolPackageRef, SkillToolApplicationError> {
    let scope = SkillToolScope::parse(scope)
        .ok_or_else(|| SkillToolApplicationError::HostDenied("source-scope".to_string()))?;
    Ok(SkillToolPackageRef {
        owner: SkillToolOwnerId::parse(skill_id)?,
        source: SkillToolSourceScope::new(scope, workspace_path)?,
        base_revision: String::new(),
        root_path: String::new(),
    })
}

pub(crate) fn revision_id(revision: &str) -> Result<SkillToolRevision, SkillToolApplicationError> {
    SkillToolRevision::parse(revision).map_err(SkillToolApplicationError::Domain)
}

pub(crate) fn revision(state: SkillToolRevisionState) -> SkillToolRevisionDto {
    SkillToolRevisionDto {
        skill_id: state.key.owner.as_str().to_string(),
        tool_id: state.key.tool.as_str().to_string(),
        canonical_id: state.key.canonical_name().unwrap_or_default(),
        revision: state.key.revision.as_str().to_string(),
        source_scope: state.key.source.scope.as_str().to_string(),
        workspace_path: state.key.source.workspace_path,
        implementation_kind: state.implementation_kind.clone(),
        base_revision: state.integrity.base_revision,
        manifest_hash: state.integrity.manifest_hash.as_str().to_string(),
        implementation_hash: state.integrity.implementation_hash.as_str().to_string(),
        capability_digest: state.integrity.capability_digest,
        capability_diff: None,
        validation: state.lifecycle.validation.as_str().to_string(),
        validation_code: state.validation_code,
        trusted: state.lifecycle.trusted,
        enabled: state.lifecycle.enabled,
        quarantined: state.lifecycle.quarantine.is_quarantined(),
        quarantine_reason: state.lifecycle.quarantine.reason().map(str::to_string),
        consecutive_failures: state.lifecycle.consecutive_failures,
        diagnostics: state
            .diagnostics
            .entries()
            .iter()
            .map(|entry| SkillToolDiagnosticDto {
                severity: entry.severity.as_str().to_string(),
                code: entry.code.clone(),
                detail: entry.detail.clone(),
            })
            .collect(),
        runtime_support: if state.implementation_kind == "wasm"
            && !crate::contexts::tooling::skill_tools::MODULE_RUNTIME_ENABLED
        {
            "module-runtime-disabled".to_string()
        } else {
            "supported".to_string()
        },
        enforcement_strength: if state.implementation_kind == "wasm" {
            "wasm-hard-limits".to_string()
        } else {
            "bounded-native-io".to_string()
        },
        created_at: state.created_at,
        updated_at: state.updated_at,
    }
}
