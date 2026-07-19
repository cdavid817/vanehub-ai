use super::dto;
use crate::contexts::tooling::skills::api as skill;

pub(super) fn scope_query(
    input: dto::SkillScopeInput,
) -> Result<skill::SkillScopeQuery, skill::SkillError> {
    Ok(skill::SkillScopeQuery {
        location: location(input.scope, input.workspace_path.as_deref())?,
    })
}

pub(super) fn key(
    skill_id: String,
    input: dto::SkillScopeInput,
) -> Result<skill::SkillKey, skill::SkillError> {
    Ok(skill::SkillKey::new(
        skill::SkillId::parse(skill_id)?,
        location(input.scope, input.workspace_path.as_deref())?,
    ))
}

pub(super) fn mount_path(value: String) -> Result<skill::SkillMountPath, skill::SkillError> {
    skill::SkillMountPath::parse(value).map_err(Into::into)
}

pub(super) fn create_request(
    input: dto::SkillMutationInput,
) -> Result<skill::SkillCreateRequest, skill::SkillError> {
    Ok(skill::SkillCreateRequest {
        id: skill::SkillId::parse(input.id)?,
        location: location(input.scope, input.workspace_path.as_deref())?,
        metadata: metadata(input.metadata)?,
        body: input.body,
        enabled: input.enabled,
        bound_agent_ids: input.bound_agent_ids,
        source: input.source.map(source_to_domain),
    })
}

pub(super) fn update_request(
    skill_id: String,
    input: dto::SkillUpdateInput,
) -> Result<skill::SkillUpdateRequest, skill::SkillError> {
    Ok(skill::SkillUpdateRequest {
        key: skill::SkillKey::new(
            skill::SkillId::parse(skill_id)?,
            location(input.scope, input.workspace_path.as_deref())?,
        ),
        metadata: metadata(input.metadata)?,
        body: input.body,
        enabled: input.enabled,
        bound_agent_ids: input.bound_agent_ids,
    })
}

pub(super) fn import_request(
    input: dto::SkillImportInput,
) -> Result<skill::SkillImportRequest, skill::SkillError> {
    Ok(skill::SkillImportRequest {
        location: location(input.scope, input.workspace_path.as_deref())?,
        source_path: input.source_path,
        enabled: input.enabled,
        bound_agent_ids: input.bound_agent_ids,
    })
}

pub(super) fn list_to_dto(result: skill::SkillListResult) -> dto::SkillListResult {
    dto::SkillListResult {
        skills: result.skills.into_iter().map(record_to_dto).collect(),
        stats: dto::SkillStats {
            total: result.stats.total,
            enabled: result.stats.enabled,
            mounted: result.stats.mounted,
        },
    }
}

pub(super) fn record_to_dto(record: skill::SkillRecord) -> dto::Skill {
    let bound_agent_ids = record.bound_agent_ids();
    dto::Skill {
        id: record.key.id.as_str().to_string(),
        scope: scope_to_dto(record.key.location.scope),
        workspace_path: record.key.location.workspace_path,
        source: source_to_dto(record.source),
        enabled: record.enabled,
        skill_dir: record.managed_source.skill_dir,
        skill_md_path: record.managed_source.skill_md_path,
        content_hash: record.managed_source.content_hash,
        metadata: metadata_to_dto(record.metadata),
        bound_agent_ids,
        bindings: record
            .bindings
            .into_iter()
            .map(|binding| dto::SkillAgentBinding {
                agent_id: binding.agent_id,
                mount_path: binding.mount_path.as_str().to_string(),
                mounted_path: binding.mounted_path,
                mounted: binding.mounted,
            })
            .collect(),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(super) fn mount_paths_to_dto(
    paths: Vec<skill::SkillAgentMountPath>,
) -> Vec<dto::SkillAgentMountPath> {
    paths
        .into_iter()
        .map(|path| dto::SkillAgentMountPath {
            agent_id: path.agent_id,
            mount_path: path.mount_path.as_str().to_string(),
            is_default: path.is_default,
        })
        .collect()
}

pub(super) fn mount_migration_to_dto(
    report: skill::SkillMountMigrationReport,
) -> dto::SkillMountMigrationReport {
    dto::SkillMountMigrationReport {
        agent_id: report.agent_id,
        old_mount_path: report.old_mount_path.as_str().to_string(),
        new_mount_path: report.new_mount_path.as_str().to_string(),
        migrated: report.migrated,
        removed: report.removed,
        overwritten: report.overwritten,
        backed_up: report.backed_up.into_iter().map(backup_to_dto).collect(),
        failed: report.failed.into_iter().map(failure_to_dto).collect(),
    }
}

pub(super) fn preview_to_dto(preview: skill::SkillPreview) -> dto::SkillPreview {
    dto::SkillPreview {
        id: preview.key.id.as_str().to_string(),
        scope: scope_to_dto(preview.key.location.scope),
        workspace_path: preview.key.location.workspace_path,
        content: preview.content,
        path: preview.path,
    }
}

pub(super) fn drift_to_dto(report: skill::SkillDriftReport) -> dto::SkillDriftReport {
    dto::SkillDriftReport {
        scope: scope_to_dto(report.location.scope),
        workspace_path: report.location.workspace_path,
        issues: report
            .issues
            .into_iter()
            .map(|issue| dto::SkillDriftIssue {
                skill_id: issue.skill_id,
                r#type: match issue.issue_type {
                    skill::SkillDriftIssueType::MissingSource => {
                        dto::SkillDriftIssueType::MissingSource
                    }
                    skill::SkillDriftIssueType::MetadataChanged => {
                        dto::SkillDriftIssueType::MetadataChanged
                    }
                    skill::SkillDriftIssueType::UnregisteredSource => {
                        dto::SkillDriftIssueType::UnregisteredSource
                    }
                    skill::SkillDriftIssueType::MissingMount => {
                        dto::SkillDriftIssueType::MissingMount
                    }
                    skill::SkillDriftIssueType::Conflict => dto::SkillDriftIssueType::Conflict,
                    skill::SkillDriftIssueType::DeletedBuiltin => {
                        dto::SkillDriftIssueType::DeletedBuiltin
                    }
                },
                agent_id: issue.agent_id,
                path: issue.path,
                message: issue.message.to_string(),
            })
            .collect(),
        drift_hash: report.drift_hash,
    }
}

pub(super) fn sync_to_dto(result: skill::SkillSyncResult) -> dto::SkillSyncResult {
    dto::SkillSyncResult {
        mounted: result.mounted,
        unmounted: result.unmounted,
        overwritten: result.overwritten,
        backed_up: result.backed_up.into_iter().map(backup_to_dto).collect(),
        restored: result.restored,
        failed: result.failed.into_iter().map(failure_to_dto).collect(),
        resolved_from: drift_to_dto(result.resolved_from),
    }
}

fn location(
    scope: dto::SkillScope,
    workspace_path: Option<&str>,
) -> Result<skill::SkillLocation, skill::SkillError> {
    skill::SkillLocation::new(scope_to_domain(scope), workspace_path).map_err(Into::into)
}

fn metadata(value: dto::SkillMetadata) -> Result<skill::SkillMetadata, skill::SkillError> {
    skill::SkillMetadata::new(
        value.id,
        value.name,
        value.description,
        value.category,
        value.version,
        value.triggers,
    )
    .map_err(Into::into)
}

fn metadata_to_dto(value: skill::SkillMetadata) -> dto::SkillMetadata {
    dto::SkillMetadata {
        id: value.id.as_str().to_string(),
        name: value.name,
        description: value.description,
        category: value.category,
        version: value.version,
        triggers: value.triggers,
    }
}

fn scope_to_domain(scope: dto::SkillScope) -> skill::SkillScope {
    match scope {
        dto::SkillScope::Global => skill::SkillScope::Global,
        dto::SkillScope::Workspace => skill::SkillScope::Workspace,
    }
}

fn scope_to_dto(scope: skill::SkillScope) -> dto::SkillScope {
    match scope {
        skill::SkillScope::Global => dto::SkillScope::Global,
        skill::SkillScope::Workspace => dto::SkillScope::Workspace,
    }
}

fn source_to_domain(source: dto::SkillSource) -> skill::SkillSource {
    match source {
        dto::SkillSource::Builtin => skill::SkillSource::Builtin,
        dto::SkillSource::User => skill::SkillSource::User,
        dto::SkillSource::Imported => skill::SkillSource::Imported,
    }
}

fn source_to_dto(source: skill::SkillSource) -> dto::SkillSource {
    match source {
        skill::SkillSource::Builtin => dto::SkillSource::Builtin,
        skill::SkillSource::User => dto::SkillSource::User,
        skill::SkillSource::Imported => dto::SkillSource::Imported,
    }
}

fn backup_to_dto(backup: skill::SkillBackupEntry) -> dto::SkillBackupEntry {
    dto::SkillBackupEntry {
        original_path: backup.original_path,
        backup_path: backup.backup_path,
    }
}

fn failure_to_dto(failure: skill::SkillFailure) -> dto::SkillFailure {
    dto::SkillFailure {
        skill_id: failure.skill_id,
        reason: failure.reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::{ManagedSkillSource, SkillAgentBinding};
    use crate::contexts::tooling::skills::domain::SkillDriftIssue;
    use serde_json::json;

    #[test]
    fn mutation_request_accepts_the_existing_camel_case_contract() {
        let input: dto::SkillMutationInput = serde_json::from_value(json!({
            "id": "fixture-skill",
            "scope": "workspace",
            "workspacePath": "D:/fixture",
            "metadata": {
                "id": "fixture-skill",
                "name": "Fixture",
                "description": "Description",
                "category": "testing",
                "version": "1.0.0",
                "triggers": ["fixture"]
            },
            "body": "Body",
            "enabled": true,
            "boundAgentIds": ["codex-cli"],
            "source": "user"
        }))
        .expect("mutation DTO");

        let request = create_request(input).expect("create request");

        assert_eq!(request.id.as_str(), "fixture-skill");
        assert_eq!(request.location.scope, skill::SkillScope::Workspace);
        assert_eq!(
            request.location.workspace_path.as_deref(),
            Some("D:/fixture")
        );
        assert_eq!(request.bound_agent_ids, vec!["codex-cli"]);
    }

    #[test]
    fn drift_response_keeps_kebab_case_type_and_camel_case_fields() {
        let report = drift_to_dto(skill::SkillDriftReport {
            location: skill::SkillLocation::new(skill::SkillScope::Global, None).expect("location"),
            issues: vec![SkillDriftIssue {
                skill_id: "fixture-skill".to_string(),
                issue_type: skill::SkillDriftIssueType::MissingMount,
                agent_id: Some("codex-cli".to_string()),
                path: Some(".codex/skills/fixture-skill".to_string()),
                message: "Agent mount is missing",
            }],
            drift_hash: "fixture-hash".to_string(),
        });

        let value = serde_json::to_value(report).expect("drift DTO");

        assert_eq!(value["issues"][0]["type"], "missing-mount");
        assert_eq!(value["issues"][0]["agentId"], "codex-cli");
        assert_eq!(value["driftHash"], "fixture-hash");
        assert!(value.get("drift_hash").is_none());
    }

    #[test]
    fn skill_response_preserves_the_complete_frontend_contract() {
        let location =
            skill::SkillLocation::new(skill::SkillScope::Global, None).expect("location");
        let record = skill::SkillRecord {
            key: skill::SkillKey::new(
                skill::SkillId::parse("fixture-skill").expect("Skill id"),
                location,
            ),
            source: skill::SkillSource::User,
            enabled: true,
            managed_source: ManagedSkillSource {
                skill_dir: "D:/home/.vanehub/skills/fixture-skill".to_string(),
                skill_md_path: "D:/home/.vanehub/skills/fixture-skill/SKILL.md".to_string(),
                content_hash: "fixture-hash".to_string(),
            },
            metadata: skill::SkillMetadata::new(
                "fixture-skill",
                "Fixture",
                "Description",
                "testing",
                "1.0.0",
                vec!["fixture".to_string()],
            )
            .expect("metadata"),
            bindings: vec![SkillAgentBinding {
                agent_id: "codex-cli".to_string(),
                mount_path: skill::SkillMountPath::parse(".codex/skills").expect("mount path"),
                mounted_path: "D:/home/.codex/skills/fixture-skill".to_string(),
                mounted: true,
            }],
            created_at: "2026-07-17T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(record_to_dto(record)).expect("Skill DTO");

        assert_eq!(
            value,
            json!({
                "id": "fixture-skill",
                "scope": "global",
                "workspacePath": null,
                "source": "user",
                "enabled": true,
                "skillDir": "D:/home/.vanehub/skills/fixture-skill",
                "skillMdPath": "D:/home/.vanehub/skills/fixture-skill/SKILL.md",
                "contentHash": "fixture-hash",
                "metadata": {
                    "id": "fixture-skill",
                    "name": "Fixture",
                    "description": "Description",
                    "category": "testing",
                    "version": "1.0.0",
                    "triggers": ["fixture"]
                },
                "boundAgentIds": ["codex-cli"],
                "bindings": [{
                    "agentId": "codex-cli",
                    "mountPath": ".codex/skills",
                    "mountedPath": "D:/home/.codex/skills/fixture-skill",
                    "mounted": true
                }],
                "createdAt": "2026-07-17T00:00:00Z",
                "updatedAt": "2026-07-18T00:00:00Z"
            })
        );
    }
}
