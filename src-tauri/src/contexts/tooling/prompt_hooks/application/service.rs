use super::{
    EffectivePromptRequest, PromptAssemblyResult, PromptHookApplicationError, PromptHookClockPort,
    PromptHookCreateRequest, PromptHookGovernance, PromptHookListResult, PromptHookLogAction,
    PromptHookLogEvent, PromptHookLogLevel, PromptHookLoggingPort, PromptHookOverride,
    PromptHookPreview, PromptHookPreviewRequest, PromptHookRecord, PromptHookRepository,
    PromptHookStats, PromptHookTrace, PromptHookTraceIdPort, PromptHookTraceStatus,
    PromptHookUpdateRequest,
};
use crate::contexts::tooling::prompt_hooks::domain::{
    builtin_prompt_hooks, compare_prompt_hook_order, ensure_content_editable, ensure_deletable,
    ensure_enablement, ensure_identity_unchanged, ensure_order_available, ManagedCliAgentId,
    PromptHookBindings, PromptHookId, PromptHookManifest, PromptHookOrderSlot, PromptHookSource,
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
        let record = PromptHookRecord {
            manifest: request.manifest,
            description: request.description.trim().to_string(),
            version: 1,
            source: PromptHookSource::User,
            enabled: request.enabled,
            disableable: true,
            governance: request.governance,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository.create_user_hook(&record)?;
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
        let record = PromptHookRecord {
            manifest: request.manifest,
            description: request.description.trim().to_string(),
            version: request.version,
            source: PromptHookSource::User,
            enabled: request.enabled,
            disableable: true,
            governance: request.governance,
            created_at: current.created_at,
            updated_at: self.clock.now(),
        };
        self.repository.update_user_hook(&record)?;
        Ok(record)
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
        let rendered_content = hook
            .manifest
            .template()
            .render(request.agent_id.as_str(), &sample_input);
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
        for hook in &hooks {
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
            let content = hook
                .manifest
                .template()
                .render(request.agent_id.as_str(), &request.user_prompt);
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
    format!("{digest:x}").chars().take(16).collect()
}
