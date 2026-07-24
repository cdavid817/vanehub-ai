use super::{
    EffectivePromptRequest, PromptAssemblyResult, PromptHookApplicationError, PromptHookClockPort,
    PromptHookCreateRequest, PromptHookDraft, PromptHookExecutionObservation, PromptHookGovernance,
    PromptHookListResult, PromptHookLogAction, PromptHookLogEvent, PromptHookLogLevel,
    PromptHookLoggingPort, PromptHookOverride, PromptHookPreview, PromptHookPreviewRequest,
    PromptHookPublicationKind, PromptHookRecord, PromptHookRepository, PromptHookSnapshot,
    PromptHookStats, PromptHookTrace, PromptHookTraceIdPort, PromptHookTraceStatus,
    PromptHookUpdateRequest, PromptHookVariable, PromptHookVersion, PromptHookVersionHistory,
    PublishPromptHookRequest, RollbackPromptHookRequest, SavePromptHookDraftRequest,
};
use crate::contexts::tooling::prompt_hooks::domain::{
    builtin_prompt_hooks, compare_prompt_hook_order, ensure_content_editable, ensure_deletable,
    ensure_enablement, ensure_identity_unchanged, ensure_order_available, ManagedCliAgentId,
    PromptHookBindings, PromptHookId, PromptHookManifest, PromptHookOrderSlot, PromptHookSource,
    PromptHookVariableDefinition, PROMPT_HOOK_VARIABLES,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

const TRACE_RETENTION_LIMIT: usize = 50;
const BUILTIN_TIMESTAMP: &str = "2026-07-18T00:00:00Z";

#[derive(Clone)]
pub(crate) struct PromptHookApplicationService {
    repository: Arc<dyn PromptHookRepository>,
    clock: Arc<dyn PromptHookClockPort>,
    trace_ids: Arc<dyn PromptHookTraceIdPort>,
    logging: Arc<dyn PromptHookLoggingPort>,
}

impl PromptHookApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn PromptHookRepository>,
        clock: Arc<dyn PromptHookClockPort>,
        trace_ids: Arc<dyn PromptHookTraceIdPort>,
        logging: Arc<dyn PromptHookLoggingPort>,
    ) -> Self {
        Self {
            repository,
            clock,
            trace_ids,
            logging,
        }
    }

    pub(crate) fn list_hooks(&self) -> Result<PromptHookListResult, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        Ok(PromptHookListResult {
            stats: stats_for_hooks(&hooks),
            hooks,
        })
    }

    pub(crate) fn create_hook(
        &self,
        request: PromptHookCreateRequest,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let hook_id = request.manifest.id().as_str().to_string();
        let result = self.create_hook_work(request);
        self.observe(PromptHookLogAction::Create, Some(hook_id), None, result)
    }

    pub(crate) fn update_hook(
        &self,
        request: PromptHookUpdateRequest,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let hook_id = request.hook_id.as_str().to_string();
        let result = self.update_hook_work(request);
        self.observe(PromptHookLogAction::Update, Some(hook_id), None, result)
    }

    pub(crate) fn delete_hook(
        &self,
        hook_id: PromptHookId,
    ) -> Result<(), PromptHookApplicationError> {
        let log_id = hook_id.as_str().to_string();
        let result = self.delete_hook_work(&hook_id);
        self.observe(PromptHookLogAction::Delete, Some(log_id), None, result)
    }

    pub(crate) fn set_enabled(
        &self,
        hook_id: PromptHookId,
        enabled: bool,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let log_id = hook_id.as_str().to_string();
        let result = self.set_enabled_work(&hook_id, enabled);
        self.observe(PromptHookLogAction::SetEnabled, Some(log_id), None, result)
    }

    pub(crate) fn set_bindings(
        &self,
        hook_id: PromptHookId,
        bindings: PromptHookBindings,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let log_id = hook_id.as_str().to_string();
        let result = self.set_bindings_work(&hook_id, bindings);
        self.observe(PromptHookLogAction::SetBindings, Some(log_id), None, result)
    }

    pub(crate) fn preview_hook(
        &self,
        request: PromptHookPreviewRequest,
    ) -> Result<PromptHookPreview, PromptHookApplicationError> {
        let hook_id = request.hook_id.as_str().to_string();
        let agent_id = request.agent_id.as_str().to_string();
        let result = self.preview_hook_work(request);
        self.observe(
            PromptHookLogAction::Preview,
            Some(hook_id),
            Some(agent_id),
            result,
        )
    }

    pub(crate) fn assemble_prompt(
        &self,
        request: EffectivePromptRequest,
    ) -> Result<PromptAssemblyResult, PromptHookApplicationError> {
        let agent_id = request.agent_id.as_str().to_string();
        let result = self.assemble_prompt_work(request);
        self.observe(PromptHookLogAction::Assemble, None, Some(agent_id), result)
    }

    pub(crate) fn list_traces(
        &self,
        limit: i64,
    ) -> Result<Vec<PromptHookTrace>, PromptHookApplicationError> {
        self.repository.list_traces(limit.clamp(1, 100) as usize)
    }

    pub(crate) fn list_variables(&self) -> Vec<PromptHookVariable> {
        let variables: &[PromptHookVariableDefinition] = &PROMPT_HOOK_VARIABLES;
        variables
            .iter()
            .map(|definition| PromptHookVariable {
                name: definition.name.to_string(),
                token: format!("{{{{{}}}}}", definition.name),
                description_key: definition.description_key.to_string(),
                availability_key: definition.availability_key.to_string(),
                example: definition.example.to_string(),
                aliases: match definition.name {
                    "agent_id" => vec!["agentId".to_string()],
                    "sample_input" => vec!["sampleInput".to_string()],
                    _ => Vec::new(),
                },
            })
            .collect()
    }

    pub(crate) fn save_draft(
        &self,
        request: SavePromptHookDraftRequest,
    ) -> Result<PromptHookDraft, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, &request.hook_id)?;
        ensure_content_editable(current.source)?;
        ensure_identity_unchanged(
            request.hook_id.as_str(),
            request.snapshot.manifest.id().as_str(),
        )?;
        ensure_manifest_order_available(
            &request.snapshot.manifest,
            &hooks,
            Some(&request.hook_id),
        )?;
        let previous = self.repository.get_draft(&request.hook_id)?;
        if previous.as_ref().map(|draft| draft.revision) != request.expected_revision {
            return Err(PromptHookApplicationError::Conflict(format!(
                "{}:stale-revision",
                request.hook_id.as_str()
            )));
        }
        let now = self.clock.now();
        let draft = PromptHookDraft {
            hook_id: request.hook_id,
            revision: previous.as_ref().map_or(1, |draft| draft.revision + 1),
            snapshot: request.snapshot,
            created_at: previous.map_or_else(|| now.clone(), |draft| draft.created_at),
            updated_at: now,
        };
        self.repository
            .save_draft(&draft, request.expected_revision)?;
        Ok(draft)
    }

    pub(crate) fn publish(
        &self,
        request: PublishPromptHookRequest,
    ) -> Result<PromptHookVersion, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, &request.hook_id)?;
        ensure_content_editable(current.source)?;
        let draft = self
            .repository
            .get_draft(&request.hook_id)?
            .ok_or_else(|| {
                PromptHookApplicationError::NotFound(format!("{}:draft", request.hook_id.as_str()))
            })?;
        if draft.revision != request.expected_draft_revision {
            return Err(PromptHookApplicationError::Conflict(format!(
                "{}:stale-revision",
                request.hook_id.as_str()
            )));
        }
        draft.snapshot.manifest.template().validate_variables()?;
        let next_version = self
            .repository
            .list_versions(&request.hook_id, 1)?
            .first()
            .map_or(current.version.max(0) + 1, |version| version.version + 1);
        let version = PromptHookVersion {
            hook_id: request.hook_id,
            version: next_version,
            content_hash: snapshot_hash(&draft.snapshot),
            snapshot: draft.snapshot,
            publication_kind: PromptHookPublicationKind::Publish,
            rollback_from_version: None,
            published_at: self.clock.now(),
        };
        self.repository.publish_draft(
            &version,
            request.expected_draft_revision,
            request.expected_published_version,
        )?;
        Ok(version)
    }

    pub(crate) fn version_history(
        &self,
        hook_id: PromptHookId,
    ) -> Result<PromptHookVersionHistory, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, &hook_id)?;
        if current.source == PromptHookSource::Builtin {
            let version = PromptHookVersion {
                hook_id: hook_id.clone(),
                version: current.version,
                snapshot: snapshot_from_record(current),
                content_hash: snapshot_hash(&snapshot_from_record(current)),
                publication_kind: PromptHookPublicationKind::Publish,
                rollback_from_version: None,
                published_at: current.updated_at.clone(),
            };
            return Ok(PromptHookVersionHistory {
                hook_id,
                published_version: Some(current.version),
                draft: None,
                versions: vec![version],
                evaluations: self.repository.evaluation_summaries(current.id(), 100)?,
            });
        }
        Ok(PromptHookVersionHistory {
            hook_id: hook_id.clone(),
            published_version: (current.version > 0).then_some(current.version),
            draft: self.repository.get_draft(&hook_id)?,
            versions: self.repository.list_versions(&hook_id, 100)?,
            evaluations: self.repository.evaluation_summaries(&hook_id, 100)?,
        })
    }

    pub(crate) fn rollback(
        &self,
        request: RollbackPromptHookRequest,
    ) -> Result<PromptHookVersion, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, &request.hook_id)?;
        ensure_content_editable(current.source)?;
        let versions = self.repository.list_versions(&request.hook_id, 100)?;
        let target = versions
            .iter()
            .find(|version| version.version == request.version)
            .ok_or_else(|| {
                PromptHookApplicationError::NotFound(format!(
                    "{}:version-{}",
                    request.hook_id.as_str(),
                    request.version
                ))
            })?;
        target.snapshot.manifest.template().validate_variables()?;
        let version = PromptHookVersion {
            hook_id: request.hook_id,
            version: versions
                .first()
                .map_or(current.version.max(0) + 1, |version| version.version + 1),
            snapshot: target.snapshot.clone(),
            content_hash: target.content_hash.clone(),
            publication_kind: PromptHookPublicationKind::Rollback,
            rollback_from_version: Some(target.version),
            published_at: self.clock.now(),
        };
        self.repository
            .publish_rollback(&version, request.expected_published_version)?;
        Ok(version)
    }

    pub(crate) fn record_execution_observations(
        &self,
        observations: &[PromptHookExecutionObservation],
    ) -> Result<(), PromptHookApplicationError> {
        self.repository.save_execution_observations(observations)
    }

    fn create_hook_work(
        &self,
        request: PromptHookCreateRequest,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        if hooks.iter().any(|hook| hook.id() == request.manifest.id()) {
            return Err(PromptHookApplicationError::Conflict(
                request.manifest.id().as_str().to_string(),
            ));
        }
        ensure_manifest_order_available(&request.manifest, &hooks, None)?;
        let now = self.clock.now();
        let snapshot = PromptHookSnapshot {
            manifest: request.manifest.clone(),
            description: request.description.trim().to_string(),
            enabled: request.enabled,
            governance: request.governance.clone(),
        };
        let record = PromptHookRecord {
            manifest: request.manifest,
            description: request.description.trim().to_string(),
            version: 0,
            source: PromptHookSource::User,
            enabled: false,
            disableable: true,
            governance: request.governance,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        self.repository.create_user_draft(
            &record,
            &PromptHookDraft {
                hook_id: record.id().clone(),
                revision: 1,
                snapshot,
                created_at: now.clone(),
                updated_at: now,
            },
        )?;
        Ok(record)
    }

    fn update_hook_work(
        &self,
        request: PromptHookUpdateRequest,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        ensure_identity_unchanged(request.hook_id.as_str(), request.manifest.id().as_str())?;
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, &request.hook_id)?.clone();
        ensure_content_editable(current.source)?;
        ensure_manifest_order_available(&request.manifest, &hooks, Some(&request.hook_id))?;
        let previous = self.repository.get_draft(&request.hook_id)?;
        let now = self.clock.now();
        let draft = PromptHookDraft {
            hook_id: request.hook_id,
            revision: previous.as_ref().map_or(1, |draft| draft.revision + 1),
            snapshot: PromptHookSnapshot {
                manifest: request.manifest,
                description: request.description.trim().to_string(),
                enabled: request.enabled,
                governance: request.governance,
            },
            created_at: previous
                .as_ref()
                .map_or_else(|| now.clone(), |draft| draft.created_at.clone()),
            updated_at: now,
        };
        self.repository
            .save_draft(&draft, previous.map(|draft| draft.revision))?;
        Ok(current)
    }

    fn delete_hook_work(&self, hook_id: &PromptHookId) -> Result<(), PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let current = find_record(&hooks, hook_id)?;
        ensure_deletable(current.source)?;
        self.repository.delete_user_hook(hook_id)
    }

    fn set_enabled_work(
        &self,
        hook_id: &PromptHookId,
        enabled: bool,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let mut current = find_record(&hooks, hook_id)?.clone();
        ensure_enablement(current.disableable, enabled)?;
        let updated_at = self.clock.now();
        match current.source {
            PromptHookSource::User => {
                self.repository
                    .set_user_enabled(hook_id, enabled, &updated_at)?;
            }
            PromptHookSource::Builtin => {
                self.repository.save_builtin_override(&PromptHookOverride {
                    hook_id: hook_id.clone(),
                    enabled,
                    bindings: current.manifest.bindings().clone(),
                    updated_at: updated_at.clone(),
                })?;
            }
        }
        current.enabled = enabled;
        current.updated_at = updated_at;
        Ok(current)
    }

    fn set_bindings_work(
        &self,
        hook_id: &PromptHookId,
        bindings: PromptHookBindings,
    ) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let mut current = find_record(&hooks, hook_id)?.clone();
        let updated_at = self.clock.now();
        match current.source {
            PromptHookSource::User => {
                self.repository
                    .set_user_bindings(hook_id, &bindings, &updated_at)?;
            }
            PromptHookSource::Builtin => {
                self.repository.save_builtin_override(&PromptHookOverride {
                    hook_id: hook_id.clone(),
                    enabled: current.enabled,
                    bindings: bindings.clone(),
                    updated_at: updated_at.clone(),
                })?;
            }
        }
        current.manifest = current.manifest.with_bindings(bindings);
        current.updated_at = updated_at;
        Ok(current)
    }

    fn preview_hook_work(
        &self,
        request: PromptHookPreviewRequest,
    ) -> Result<PromptHookPreview, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let hook = find_record(&hooks, &request.hook_id)?;
        let sample_input = request
            .sample_input
            .unwrap_or_else(|| "Preview request".to_string());
        let current_time = self.clock.now();
        let rendered_content = hook.manifest.template().render(
            crate::contexts::tooling::prompt_hooks::domain::PromptHookTemplateContext {
                agent_id: request.agent_id.as_str(),
                agent_name: request.agent_id.display_name(),
                current_time: &current_time,
                sample_input: &sample_input,
                session_id: "session-preview",
            },
        )?;
        let (status, content, reason) = if hook.enabled {
            (
                PromptHookTraceStatus::Fired,
                Some(rendered_content.as_str()),
                None,
            )
        } else {
            (PromptHookTraceStatus::Disabled, None, Some("disabled"))
        };
        let trace =
            vec![self.trace_for_hook(hook, status, content, request.agent_id, None, reason)];
        self.repository.save_traces(&trace, TRACE_RETENTION_LIMIT)?;
        Ok(PromptHookPreview {
            hook_id: request.hook_id,
            agent_id: request.agent_id,
            rendered_content,
            trace,
        })
    }

    fn assemble_prompt_work(
        &self,
        request: EffectivePromptRequest,
    ) -> Result<PromptAssemblyResult, PromptHookApplicationError> {
        let hooks = self.effective_hooks()?;
        let mut rendered_parts = Vec::new();
        let mut traces = Vec::with_capacity(hooks.len());
        let current_time = self.clock.now();
        for hook in &hooks {
            if hook.source == PromptHookSource::User && hook.version <= 0 {
                traces.push(self.trace_for_hook(
                    hook,
                    PromptHookTraceStatus::Skipped,
                    None,
                    request.agent_id,
                    request.session_id.as_deref(),
                    Some("unpublished"),
                ));
                continue;
            }
            if !hook.enabled {
                traces.push(self.trace_for_hook(
                    hook,
                    PromptHookTraceStatus::Disabled,
                    None,
                    request.agent_id,
                    request.session_id.as_deref(),
                    Some("disabled"),
                ));
                continue;
            }
            if !hook.manifest.bindings().contains(request.agent_id) {
                traces.push(self.trace_for_hook(
                    hook,
                    PromptHookTraceStatus::Skipped,
                    None,
                    request.agent_id,
                    request.session_id.as_deref(),
                    Some("unbound-cli"),
                ));
                continue;
            }
            let content = hook.manifest.template().render(
                crate::contexts::tooling::prompt_hooks::domain::PromptHookTemplateContext {
                    agent_id: request.agent_id.as_str(),
                    agent_name: request.agent_id.display_name(),
                    current_time: &current_time,
                    sample_input: &request.user_prompt,
                    session_id: request.session_id.as_deref().unwrap_or_default(),
                },
            )?;
            traces.push(self.trace_for_hook(
                hook,
                PromptHookTraceStatus::Fired,
                Some(&content),
                request.agent_id,
                request.session_id.as_deref(),
                None,
            ));
            rendered_parts.push(content);
        }
        rendered_parts.push(request.user_prompt);
        let effective_prompt = rendered_parts
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        self.repository
            .save_traces(&traces, TRACE_RETENTION_LIMIT)?;
        Ok(PromptAssemblyResult {
            effective_prompt,
            trace: traces,
        })
    }

    fn effective_hooks(&self) -> Result<Vec<PromptHookRecord>, PromptHookApplicationError> {
        let overrides = self
            .repository
            .list_builtin_overrides()?
            .into_iter()
            .map(|override_record| (override_record.hook_id.clone(), override_record))
            .collect::<HashMap<_, _>>();
        let mut hooks = builtin_prompt_hooks()
            .iter()
            .map(builtin_record)
            .collect::<Result<Vec<_>, _>>()?;
        for hook in &mut hooks {
            if let Some(override_record) = overrides.get(hook.id()) {
                hook.enabled = override_record.enabled;
                hook.manifest = hook
                    .manifest
                    .clone()
                    .with_bindings(override_record.bindings.clone());
                hook.updated_at.clone_from(&override_record.updated_at);
            }
        }
        hooks.extend(self.repository.list_user_hooks()?);
        hooks.sort_by(|left, right| {
            compare_prompt_hook_order(
                (
                    left.manifest.stage(),
                    left.manifest.category(),
                    left.manifest.order().value(),
                    left.id().as_str(),
                ),
                (
                    right.manifest.stage(),
                    right.manifest.category(),
                    right.manifest.order().value(),
                    right.id().as_str(),
                ),
            )
        });
        Ok(hooks)
    }

    fn trace_for_hook(
        &self,
        hook: &PromptHookRecord,
        status: PromptHookTraceStatus,
        content: Option<&str>,
        agent_id: ManagedCliAgentId,
        session_id: Option<&str>,
        reason: Option<&str>,
    ) -> PromptHookTrace {
        PromptHookTrace {
            id: self.trace_ids.next_trace_id(),
            hook_id: hook.id().clone(),
            category: hook.manifest.category(),
            stage: hook.manifest.stage(),
            status,
            version: (status == PromptHookTraceStatus::Fired).then_some(hook.version),
            content_hash: content.map(hash_content),
            token_estimate: content.map(|value| value.chars().count().div_ceil(4) as i64),
            reason: reason.map(str::to_string),
            agent_id: Some(agent_id),
            session_id: session_id.map(str::to_string),
            created_at: self.clock.now(),
        }
    }

    fn observe<T>(
        &self,
        action: PromptHookLogAction,
        hook_id: Option<String>,
        agent_id: Option<String>,
        result: Result<T, PromptHookApplicationError>,
    ) -> Result<T, PromptHookApplicationError> {
        let (level, message) = match &result {
            Ok(_) => (
                PromptHookLogLevel::Info,
                format!("Prompt Hook {} completed", action.as_str()),
            ),
            Err(error) => (PromptHookLogLevel::Error, error.to_string()),
        };
        self.logging.record(&PromptHookLogEvent {
            action,
            level,
            hook_id,
            agent_id,
            message,
        });
        result
    }
}

fn builtin_record(
    definition: &crate::contexts::tooling::prompt_hooks::domain::BuiltinPromptHookDefinition,
) -> Result<PromptHookRecord, PromptHookApplicationError> {
    Ok(PromptHookRecord {
        manifest: PromptHookManifest::new(
            definition.id,
            definition.name,
            definition.category,
            definition.stage,
            definition.order,
            definition.template_body,
            &PromptHookBindings::all().to_strings(),
        )?,
        description: definition.description.to_string(),
        version: 1,
        source: PromptHookSource::Builtin,
        enabled: definition.enabled,
        disableable: definition.disableable,
        governance: default_governance(definition.disableable),
        created_at: BUILTIN_TIMESTAMP.to_string(),
        updated_at: BUILTIN_TIMESTAMP.to_string(),
    })
}

fn default_governance(disableable: bool) -> PromptHookGovernance {
    PromptHookGovernance {
        safety_tier: "readonly".to_string(),
        transparency_tier: if disableable {
            "opt-in-view".to_string()
        } else {
            "visible-by-default".to_string()
        },
        governance_tier: if disableable {
            "human-gated".to_string()
        } else {
            "immutable".to_string()
        },
    }
}

fn stats_for_hooks(hooks: &[PromptHookRecord]) -> PromptHookStats {
    PromptHookStats {
        total: hooks.len(),
        enabled: hooks.iter().filter(|hook| hook.enabled).count(),
        builtin: hooks
            .iter()
            .filter(|hook| hook.source == PromptHookSource::Builtin)
            .count(),
        user: hooks
            .iter()
            .filter(|hook| hook.source == PromptHookSource::User)
            .count(),
    }
}

fn find_record<'a>(
    hooks: &'a [PromptHookRecord],
    hook_id: &PromptHookId,
) -> Result<&'a PromptHookRecord, PromptHookApplicationError> {
    hooks
        .iter()
        .find(|hook| hook.id() == hook_id)
        .ok_or_else(|| PromptHookApplicationError::NotFound(hook_id.as_str().to_string()))
}

fn ensure_manifest_order_available(
    manifest: &PromptHookManifest,
    hooks: &[PromptHookRecord],
    excluded_id: Option<&PromptHookId>,
) -> Result<(), PromptHookApplicationError> {
    let occupied = hooks
        .iter()
        .filter(|hook| excluded_id != Some(hook.id()))
        .map(|hook| hook.manifest.order_slot())
        .collect::<Vec<PromptHookOrderSlot>>();
    ensure_order_available(manifest.order_slot(), &occupied).map_err(Into::into)
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    bytes_to_hex(&digest).chars().take(16).collect()
}

fn snapshot_from_record(record: &PromptHookRecord) -> PromptHookSnapshot {
    PromptHookSnapshot {
        manifest: record.manifest.clone(),
        description: record.description.clone(),
        enabled: record.enabled,
        governance: record.governance.clone(),
    }
}

fn snapshot_hash(snapshot: &PromptHookSnapshot) -> String {
    hash_content(&format!(
        "{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}\u{001f}{}",
        snapshot.manifest.id().as_str(),
        snapshot.manifest.name().as_str(),
        snapshot.description,
        snapshot.manifest.category().as_str(),
        snapshot.manifest.stage().as_str(),
        snapshot.manifest.order().value(),
        snapshot.manifest.template().as_str(),
        snapshot.enabled,
    ))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
