//! Translation between the wire shapes and the domain.
//!
//! One direction parses and one renders, and both live here so a screen never has to reconstruct a
//! domain rule. An unknown string is rejected rather than defaulted: a mode, scope or status this
//! build does not understand is not safely readable as the most permissive one, and defaulting
//! would turn a typo in a caller into a silently wider operation.

use super::dto;
use crate::commands::error::CommandError;
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::personalization::application::{
    AgentCapabilityEntry, CreateMemoryInput, EffectivePreview, ResolutionRequest, ReviewRequest,
    UpdateMemoryPatch, WorkspaceIdentityRequest,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, MemoryAudience, MemoryCandidate, MemoryCandidateOperation,
    MemoryCursor, MemoryId, MemoryPage, MemoryProvenance, MemoryQuery, MemoryRecord, MemoryScope,
    MemoryScopeFilter, MemorySensitivity, MemorySource, MemoryStatus, MemoryType,
    PersonalizationPolicyPatch, PersonalizationPolicyRecord, PersonalizationPolicyScope,
    PolicyToggle, ReconcileMemoryOutcome, ResetMemoryOutcome, ResetMemoryPreview, ReviewAction,
    ReviewOutcome, SessionId, SessionPersonalizationMode, WorkspaceIdentity, WorkspaceKey,
    WorkspaceKind,
};

fn invalid(field: &str, value: &str) -> CommandError {
    CommandError::validation(format!("unsupported {field}: {value}"))
}

fn agent_id(value: &str) -> Result<AgentId, CommandError> {
    AgentId::parse(value).map_err(|error| CommandError::validation(error.to_string()))
}

fn workspace_key(value: &str) -> Result<WorkspaceKey, CommandError> {
    WorkspaceKey::parse(value).map_err(|error| CommandError::validation(error.to_string()))
}

pub(super) fn memory_id(value: &str) -> Result<MemoryId, CommandError> {
    MemoryId::parse(value).map_err(|error| CommandError::validation(error.to_string()))
}

fn toggle(value: &str) -> Result<PolicyToggle, CommandError> {
    match value {
        "inherit" => Ok(PolicyToggle::Inherit),
        "enabled" => Ok(PolicyToggle::Enabled),
        "disabled" => Ok(PolicyToggle::Disabled),
        other => Err(invalid("toggle", other)),
    }
}

fn merge_mode(value: &str) -> Result<InstructionMergeMode, CommandError> {
    match value {
        "inherit" => Ok(InstructionMergeMode::Inherit),
        "append" => Ok(InstructionMergeMode::Append),
        "replace" => Ok(InstructionMergeMode::Replace),
        "disabled" => Ok(InstructionMergeMode::Disabled),
        other => Err(invalid("instruction merge mode", other)),
    }
}

fn memory_type(value: &str) -> Result<MemoryType, CommandError> {
    match value {
        "user" => Ok(MemoryType::User),
        "feedback" => Ok(MemoryType::Feedback),
        "project" => Ok(MemoryType::Project),
        "reference" => Ok(MemoryType::Reference),
        // `untyped` is a migration outcome, not something a caller may choose. Accepting it would
        // let a screen create a record claiming to predate the taxonomy.
        other => Err(invalid("memory type", other)),
    }
}

fn memory_status(value: &str) -> Result<MemoryStatus, CommandError> {
    match value {
        "active" => Ok(MemoryStatus::Active),
        "archived" => Ok(MemoryStatus::Archived),
        // `candidate` is a queue entry, not a memory a list may return. Asking for one
        // through the memory query would surface unapproved text as though it were stored.
        "candidate" => Err(invalid("memory status", "candidate")),
        other => Err(invalid("memory status", other)),
    }
}

fn sensitivity(value: &str) -> Result<MemorySensitivity, CommandError> {
    match value {
        "normal" => Ok(MemorySensitivity::Normal),
        "sensitive" => Ok(MemorySensitivity::Sensitive),
        other => Err(invalid("sensitivity", other)),
    }
}

pub(super) fn policy_scope(
    scope_kind: &str,
    agent: Option<&str>,
    workspace: Option<&str>,
) -> Result<PersonalizationPolicyScope, CommandError> {
    match (scope_kind, agent, workspace) {
        ("global", _, _) => Ok(PersonalizationPolicyScope::Global),
        ("agent", Some(agent), _) => Ok(PersonalizationPolicyScope::Agent {
            agent_id: agent_id(agent)?,
        }),
        ("workspace", _, Some(workspace)) => Ok(PersonalizationPolicyScope::Workspace {
            workspace_key: workspace_key(workspace)?,
        }),
        ("workspace-agent", Some(agent), Some(workspace)) => {
            Ok(PersonalizationPolicyScope::WorkspaceAgent {
                agent_id: agent_id(agent)?,
                workspace_key: workspace_key(workspace)?,
            })
        }
        // A scope missing the key it is named after addresses nothing, and guessing which layer
        // was meant would edit one the user did not open.
        (kind, _, _) => Err(invalid("policy scope", kind)),
    }
}

pub(super) fn scope_filter(
    scope_kind: Option<&str>,
    workspace: Option<&str>,
) -> Result<MemoryScopeFilter, CommandError> {
    match (scope_kind, workspace) {
        (None, _) | (Some("any"), _) => Ok(MemoryScopeFilter::Any),
        (Some("global"), _) => Ok(MemoryScopeFilter::GlobalOnly),
        (Some("workspace"), Some(workspace)) => Ok(MemoryScopeFilter::Workspace {
            workspace_key: workspace_key(workspace)?,
        }),
        (Some(kind), _) => Err(invalid("scope filter", kind)),
    }
}

pub(super) fn policy_to_dto(
    record: &PersonalizationPolicyRecord,
) -> dto::PersonalizationPolicyView {
    dto::PersonalizationPolicyView {
        scope_kind: record.scope().scope_kind().to_string(),
        scope_key: record.scope().scope_key(),
        revision: record.revision(),
        instruction_merge_mode: record.instruction_merge_mode().as_str().to_string(),
        about_user: record.about_user().to_string(),
        style_rules: record.style_rules().to_string(),
        memory_read_mode: record.memory_read_mode().as_str().to_string(),
        explicit_save_mode: record.explicit_save_mode().as_str().to_string(),
        automatic_extraction_mode: record.automatic_extraction_mode().as_str().to_string(),
        global_memory_access_mode: record.global_memory_access_mode().as_str().to_string(),
    }
}

pub(super) fn policy_patch(
    input: &dto::PersonalizationPolicyPatchInput,
) -> Result<PersonalizationPolicyPatch, CommandError> {
    Ok(PersonalizationPolicyPatch {
        instruction_merge_mode: input
            .instruction_merge_mode
            .as_deref()
            .map(merge_mode)
            .transpose()?,
        about_user: input.about_user.clone(),
        style_rules: input.style_rules.clone(),
        memory_read_mode: input.memory_read_mode.as_deref().map(toggle).transpose()?,
        explicit_save_mode: input
            .explicit_save_mode
            .as_deref()
            .map(toggle)
            .transpose()?,
        automatic_extraction_mode: input
            .automatic_extraction_mode
            .as_deref()
            .map(toggle)
            .transpose()?,
        global_memory_access_mode: input
            .global_memory_access_mode
            .as_deref()
            .map(toggle)
            .transpose()?,
    })
}

pub(super) fn capabilities_to_dto(
    entries: Vec<AgentCapabilityEntry>,
) -> Vec<dto::AgentCapabilityView> {
    entries
        .into_iter()
        .map(|entry| dto::AgentCapabilityView {
            agent_id: entry.agent_id.as_str().to_string(),
            display_name: entry.display_name,
            supports_custom_instructions: entry.capabilities.supports_custom_instructions,
            supports_memory_index: entry.capabilities.supports_memory_index,
            supports_selected_memory_bodies: entry.capabilities.supports_selected_memory_bodies,
            supports_automatic_extraction: entry.capabilities.supports_automatic_extraction,
        })
        .collect()
}

pub(super) fn resolution_request(
    input: dto::EffectivePreviewInput,
) -> Result<ResolutionRequest, CommandError> {
    let workspace = match (input.workspace_key, input.workspace_display_path) {
        (Some(key), display) => Some(WorkspaceIdentity::new(
            workspace_key(&key)?,
            display.unwrap_or_default(),
            WorkspaceKind::Local,
        )),
        (None, _) => None,
    };
    let session_mode = match input.session_mode.as_deref() {
        None | Some("standard") => SessionPersonalizationMode::Standard,
        Some("project-only") => SessionPersonalizationMode::ProjectOnly,
        Some("temporary") => SessionPersonalizationMode::Temporary,
        Some(other) => return Err(invalid("session mode", other)),
    };
    Ok(ResolutionRequest {
        agent_id: agent_id(&input.agent_id)?,
        session_id: SessionId::parse(&input.session_id)
            .map_err(|error| CommandError::validation(error.to_string()))?,
        workspace,
        session_mode,
        // A preview renders stored policy. A session override is transient state a screen does not
        // hold, and inventing one here would show the user a resolution no session ever had.
        session_override: None,
    })
}

/// Composes the remote URI natively from parts the wire could carry.
///
/// The resolver accepts a URI, and building it here rather than accepting one keeps a password
/// component from ever being a thing a caller could send.
pub(super) fn workspace_request(input: dto::WorkspaceScopeInput) -> WorkspaceIdentityRequest {
    let remote_uri = input.remote.map(|remote| {
        let authority = match remote
            .user
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(user) => format!("{user}@{}", remote.host),
            None => remote.host.clone(),
        };
        let path = remote.path.trim_start_matches('/');
        format!("ssh://{authority}:{}/{path}", remote.port.unwrap_or(22))
    });
    WorkspaceIdentityRequest {
        stable_id: input.stable_id,
        project_path: input.project_path,
        worktree_path: input.worktree_path,
        remote_uri,
    }
}

pub(super) fn workspace_to_dto(identity: &WorkspaceIdentity) -> dto::WorkspaceScopeView {
    dto::WorkspaceScopeView {
        workspace_key: identity.key().as_str().to_string(),
        kind: match identity.kind() {
            WorkspaceKind::Local => "local",
            WorkspaceKind::Remote => "remote",
        }
        .to_string(),
    }
}

pub(super) fn preview_to_dto(preview: EffectivePreview) -> dto::EffectivePreviewView {
    dto::EffectivePreviewView {
        revision_token: preview.revision_token,
        instruction_mode: preview.instruction_mode.as_str().to_string(),
        included_instructions: preview
            .included_instructions
            .into_iter()
            .map(|segment| dto::PreviewSegmentView {
                field: segment.field.as_str().to_string(),
                scope_kind: segment.scope_kind.to_string(),
                scope_key: segment.scope_key,
                policy_revision: segment.policy_revision,
                merge_action: segment.merge_action.as_str().to_string(),
                redacted_text: segment.redacted_text,
                characters: segment.characters,
            })
            .collect(),
        excluded_instructions: preview
            .excluded_instructions
            .into_iter()
            .map(|segment| dto::ExcludedSegmentView {
                field: segment.field.as_str().to_string(),
                scope_kind: segment.scope_kind.to_string(),
                scope_key: segment.scope_key,
                reason: segment.reason.as_str().to_string(),
            })
            .collect(),
        memory_delivery: preview.delivery.as_str().to_string(),
        memory_read: preview.memory_access.read,
        explicit_save: preview.memory_access.explicit_save,
        automatic_extraction: preview.memory_access.automatic_extraction,
        candidate_creation: preview.memory_access.candidate_creation,
        retrieval_write: preview.memory_access.retrieval_write,
        eligible_memory_count: preview.eligible_memory_count,
        considered_memory_count: preview.considered_memory_count,
        memory_exclusions: preview
            .memory_exclusions
            .into_iter()
            .map(|exclusion| dto::MemoryExclusionView {
                reason: exclusion.reason.as_str().to_string(),
                count: exclusion.count,
            })
            .collect(),
        warnings: preview
            .warnings
            .into_iter()
            .map(|warning| warning.code.as_str().to_string())
            .collect(),
        approximate_tokens: preview.context_estimate.approximate_tokens,
        known_characters: preview.context_estimate.known_characters,
        selected_body_budget_max: preview.context_estimate.selected_body_budget_max,
        excluded_surfaces: preview
            .context_estimate
            .excluded_surfaces
            .into_iter()
            .map(str::to_string)
            .collect(),
        estimator_version: preview.context_estimate.estimator_version.to_string(),
        cli_internal_compaction_managed: preview.cli_internal_compaction_managed,
    }
}

pub(super) fn memory_query(input: dto::MemoryQueryInput) -> Result<MemoryQuery, CommandError> {
    let mut query = MemoryQuery {
        search: input.text,
        scope: scope_filter(input.scope_kind.as_deref(), input.workspace_key.as_deref())?,
        statuses: input
            .status
            .as_deref()
            .map(memory_status)
            .transpose()?
            .map(|status| vec![status])
            .unwrap_or_default(),
        memory_types: input
            .memory_type
            .as_deref()
            .map(memory_type)
            .transpose()?
            .map(|value| vec![value])
            .unwrap_or_default(),
        source_agent_id: input.source_agent_id.as_deref().map(agent_id).transpose()?,
        audience_agent_id: input
            .audience_agent_id
            .as_deref()
            .map(agent_id)
            .transpose()?,
        ..MemoryQuery::default()
    };
    if let Some(cursor) = input.cursor {
        query.cursor = Some(parse_cursor(&cursor)?);
    }
    if let Some(limit) = input.limit {
        query = query.with_page_size(limit);
    }
    Ok(query)
}

/// `sort key` and id, joined by a character neither can contain.
///
/// Opaque to the caller on purpose: a screen that built its own cursor would be depending on the
/// ordering key, which is the store's to change.
fn parse_cursor(value: &str) -> Result<MemoryCursor, CommandError> {
    let (sort_key, id) = value
        .split_once('\u{1f}')
        .ok_or_else(|| CommandError::validation("unreadable page cursor".to_string()))?;
    Ok(MemoryCursor {
        sort_key: sort_key.to_string(),
        id: memory_id(id)?,
    })
}

fn render_cursor(cursor: &MemoryCursor) -> String {
    format!("{}\u{1f}{}", cursor.sort_key, cursor.id.as_str())
}

pub(super) fn page_to_dto(page: MemoryPage) -> dto::MemoryPageView {
    dto::MemoryPageView {
        items: page
            .items
            .into_iter()
            .map(|item| dto::MemorySummaryView {
                id: item.id.as_str().to_string(),
                name: item.name,
                description: item.description,
                memory_type: item.memory_type.as_str().to_string(),
                scope_kind: item.scope_kind.to_string(),
                workspace_key: item.workspace_key.map(|key| key.as_str().to_string()),
                status: item.status.as_str().to_string(),
                source: item.source.as_str().to_string(),
                sensitivity: if item.audience_is_restricted {
                    "restricted".to_string()
                } else {
                    "normal".to_string()
                },
                revision: item.revision,
                updated_at: item.updated_at.to_rfc3339(),
            })
            .collect(),
        next_cursor: page.next_cursor.as_ref().map(render_cursor),
        total_matched: page.total_matched,
    }
}

pub(super) fn detail_to_dto(record: MemoryRecord) -> dto::MemoryDetailView {
    dto::MemoryDetailView {
        id: record.id.as_str().to_string(),
        name: record.name,
        description: record.description,
        memory_type: record.memory_type.as_str().to_string(),
        content: record.content,
        scope_kind: record.scope.kind_str().to_string(),
        workspace_key: record
            .scope
            .workspace_key()
            .map(|key| key.as_str().to_string()),
        audience_agent_ids: match record.audience {
            MemoryAudience::AllAgents => None,
            MemoryAudience::SelectedAgents { agent_ids } => Some(
                agent_ids
                    .into_iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            ),
        },
        status: record.status.as_str().to_string(),
        source: record.source.as_str().to_string(),
        sensitivity: record.sensitivity.as_str().to_string(),
        revision: record.revision,
        source_agent_id: record
            .provenance
            .source_agent_id
            .map(|id| id.as_str().to_string()),
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

pub(super) fn create_input(
    input: dto::CreateMemoryCommandInput,
) -> Result<CreateMemoryInput, CommandError> {
    let scope = match (input.scope_kind.as_str(), input.workspace_key.as_deref()) {
        ("global", _) => MemoryScope::Global,
        ("workspace", Some(key)) => MemoryScope::Workspace {
            workspace_key: workspace_key(key)?,
        },
        (kind, _) => return Err(invalid("memory scope", kind)),
    };
    let audience = match input.audience_agent_ids {
        None => MemoryAudience::AllAgents,
        Some(ids) => MemoryAudience::SelectedAgents {
            agent_ids: ids
                .iter()
                .map(|id| agent_id(id))
                .collect::<Result<Vec<_>, _>>()?,
        },
    };
    Ok(CreateMemoryInput {
        name: input.name,
        description: input.description,
        memory_type: memory_type(&input.memory_type)?,
        content: input.content,
        scope,
        audience,
        status: MemoryStatus::Active,
        // A memory created through this command is one a person wrote. Nothing else may claim it.
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    })
}

pub(super) fn update_patch(
    input: &dto::UpdateMemoryCommandInput,
) -> Result<UpdateMemoryPatch, CommandError> {
    Ok(UpdateMemoryPatch {
        name: input.name.clone(),
        description: input.description.clone(),
        memory_type: input.memory_type.as_deref().map(memory_type).transpose()?,
        content: input.content.clone(),
        // Scope and audience are deliberately not editable here: moving a memory between scopes
        // changes who can see it, which is a different operation from correcting what it says.
        scope: None,
        audience: None,
        status: input.status.as_deref().map(memory_status).transpose()?,
        sensitivity: input.sensitivity.as_deref().map(sensitivity).transpose()?,
    })
}

pub(super) fn candidates_to_dto(candidates: Vec<MemoryCandidate>) -> Vec<dto::MemoryCandidateView> {
    candidates
        .into_iter()
        .map(|candidate| {
            let (name, description, memory_type, content, target_id, expected) =
                match &candidate.operation {
                    MemoryCandidateOperation::Create(create) => (
                        Some(create.name.clone()),
                        Some(create.description.clone()),
                        Some(create.memory_type.as_str().to_string()),
                        Some(create.content.clone()),
                        None,
                        None,
                    ),
                    MemoryCandidateOperation::Update(update) => (
                        update.name.clone(),
                        update.description.clone(),
                        None,
                        update.content.clone(),
                        Some(update.target_id.as_str().to_string()),
                        Some(update.expected_target_revision),
                    ),
                    MemoryCandidateOperation::Archive(archive) => (
                        None,
                        None,
                        None,
                        None,
                        Some(archive.target_id.as_str().to_string()),
                        Some(archive.expected_target_revision),
                    ),
                };
            dto::MemoryCandidateView {
                id: candidate.id.as_str().to_string(),
                kind: candidate.operation.kind_str().to_string(),
                name,
                description,
                memory_type,
                content,
                target_id,
                expected_target_revision: expected,
                source: candidate.source.as_str().to_string(),
                source_agent_id: candidate
                    .provenance
                    .source_agent_id
                    .as_ref()
                    .map(|id| id.as_str().to_string()),
                source_session_id: candidate
                    .provenance
                    .source_session_id
                    .as_ref()
                    .map(|id| id.as_str().to_string()),
                source_message_id: candidate.provenance.source_message_id.clone(),
                created_at: candidate.created_at.to_rfc3339(),
            }
        })
        .collect()
}

/// A reviewer's scope choice, or `None` when they made none.
///
/// `None` keeps what the proposal carried. Defaulting to global would let an edit to the wording
/// widen a workspace memory to every project, which is the one change a reviewer would not expect
/// to have made.
fn memory_scope(
    scope_kind: Option<&str>,
    workspace: Option<&str>,
) -> Result<Option<MemoryScope>, CommandError> {
    match (scope_kind, workspace) {
        (None, _) => Ok(None),
        (Some("global"), _) => Ok(Some(MemoryScope::Global)),
        (Some("workspace"), Some(workspace)) => Ok(Some(MemoryScope::Workspace {
            workspace_key: workspace_key(workspace)?,
        })),
        (Some(kind), _) => Err(invalid("memory scope", kind)),
    }
}

/// An empty list is not "everyone": it is a real choice meaning no Agent may read the record.
fn audience_of(agent_ids: Vec<String>) -> Result<MemoryAudience, CommandError> {
    Ok(MemoryAudience::SelectedAgents {
        agent_ids: agent_ids
            .iter()
            .map(|value| agent_id(value))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub(super) fn review_request(
    input: dto::ReviewCandidateInput,
) -> Result<ReviewRequest, CommandError> {
    let action = match input.action.as_str() {
        "approve" => ReviewAction::Approve,
        "approve-with-edits" => ReviewAction::ApproveWithEdits {
            name: input.name,
            description: input.description,
            content: input.content,
            memory_type: input.memory_type.as_deref().map(memory_type).transpose()?,
            scope: memory_scope(input.scope_kind.as_deref(), input.workspace_key.as_deref())?,
            audience: input.audience_agent_ids.map(audience_of).transpose()?,
        },
        "reject" => ReviewAction::Reject,
        "mark-sensitive-and-archive" => ReviewAction::MarkSensitiveAndArchive,
        "merge-into" => {
            let target = input
                .merge_target_id
                .ok_or_else(|| CommandError::validation("merge needs a target".to_string()))?;
            ReviewAction::MergeInto {
                target_id: memory_id(&target)?,
                expected_target_revision: input.merge_expected_revision.ok_or_else(|| {
                    CommandError::validation("merge needs the target's revision".to_string())
                })?,
            }
        }
        other => return Err(invalid("review action", other)),
    };
    Ok(ReviewRequest {
        candidate_id: memory_id(&input.candidate_id)?,
        action,
    })
}

pub(super) fn review_outcome_to_dto(outcome: ReviewOutcome) -> dto::ReviewOutcomeView {
    dto::ReviewOutcomeView {
        candidate_id: outcome.candidate_id.as_str().to_string(),
        status: match outcome.status {
            crate::contexts::personalization::domain::CandidateReviewStatus::Pending => "pending",
            crate::contexts::personalization::domain::CandidateReviewStatus::Approved => "approved",
            crate::contexts::personalization::domain::CandidateReviewStatus::Rejected => "rejected",
        }
        .to_string(),
        resulting_memory_id: outcome
            .resulting_memory_id
            .map(|id| id.as_str().to_string()),
        retained_content: outcome.retained_content,
    }
}

/// The statuses an "include archived" checkbox stands for.
///
/// Preview and execute have to agree exactly, because the token issued by one names the statuses
/// it authorises and the other is refused if they differ. Deriving both from here is what keeps
/// them from drifting apart.
pub(super) fn reset_statuses(include_archived: bool) -> Vec<MemoryStatus> {
    match include_archived {
        true => vec![MemoryStatus::Active, MemoryStatus::Archived],
        false => vec![MemoryStatus::Active],
    }
}

pub(super) fn reset_preview_to_dto(preview: &ResetMemoryPreview) -> dto::ResetPreviewView {
    dto::ResetPreviewView {
        confirmation_token: preview.token.value.clone(),
        matched: preview.matched,
        global: preview.matched_global,
        workspace: preview.matched_workspace,
        candidates: preview.matched_candidates,
        malformed: preview.matched_malformed,
    }
}

pub(super) fn reset_outcome_to_dto(outcome: ResetMemoryOutcome) -> dto::MaintenanceResultView {
    dto::MaintenanceResultView {
        matched: outcome.matched,
        deleted_files: outcome.deleted_files,
        removed_projection_rows: outcome.deleted_projection_rows,
        revoked_retrieval_entries: outcome.revoked_retrieval_entries,
        quarantined: outcome.removed_quarantine_entries,
        failures: outcome
            .failures
            .iter()
            .map(|failure| failure.phase.as_str().to_string())
            .collect(),
    }
}

pub(super) fn reconcile_to_dto(outcome: ReconcileMemoryOutcome) -> dto::MaintenanceResultView {
    dto::MaintenanceResultView {
        matched: outcome.scanned_entries,
        // Reconciliation removes nothing authoritative; it rebuilds what is derived from what is.
        deleted_files: 0,
        removed_projection_rows: outcome.rebuilt_projection_rows,
        revoked_retrieval_entries: outcome.revoked_orphan_retrieval_entries,
        quarantined: outcome.quarantined_entries,
        failures: outcome
            .failures
            .iter()
            .map(|failure| failure.phase.as_str().to_string())
            .collect(),
    }
}

pub(super) fn health_to_dto(api: &PersonalizationApi) -> dto::PersonalizationHealthView {
    let health = api.memory_health();
    dto::PersonalizationHealthView {
        state: health.as_str().to_string(),
        memory_available: health.allows_memory_use(),
        // A queue a user can read is not a memory a runtime can act on, so an unreadable count
        // reports zero rather than failing the whole health call.
        pending_candidates: api.pending_memory_candidate_count().unwrap_or(0),
    }
}
