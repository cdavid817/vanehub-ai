//! Tool catalog resolution, system prompt assembly, personalization, and memory sections.

use super::super::memory_surfaced::{mark_surfaced, unsurfaced_candidates};
use super::super::skill_tool_catalog_adapter::{
    resolve_skill_tool_catalog, ResolvedSkillToolCatalog,
};
use super::super::tools::{task_list_prompt_section, ToolExecutionOutcome};
use super::{SKILL_AGGREGATE_CHARACTER_BUDGET, SKILL_PER_ITEM_CHARACTER_BUDGET};
use crate::contexts::agent_runtime::application::MemorySource;
use crate::contexts::agent_runtime::application::{
    ask_user_question_tool_definition, code_intelligence_tool_definitions,
    delegate_utility_skill_tool_definition, plan_mode_tool_catalog, recall_tool_definition,
    search_code_tool_definition, tool_catalog, AgentCandidateSubmission, AgentClockPort,
    AgentCodeIntelligenceContext, AgentCodeIntelligencePort, AgentCoreInstructionsPort, AgentLog,
    AgentLogLevel, AgentLoggingPort, AgentMcpToolPort, AgentMemory, AgentMemoryDelivery,
    AgentMemoryProposal, AgentMemoryRef, AgentMemorySelectionPort, AgentPersonalizationSnapshot,
    AgentPersonalizationSnapshotPort, AgentProposalOrigin, AgentRetrievalPort, AgentSkillPort,
    ApiProviderConfig, BoundSkillPrompt, GenerationPersonalizationContext,
    GenerationProcessRequest, NativeToolExecutionMode, NativeToolRegistry, ToolDefinition,
    ToolEligibilityContext, UtilityDelegationApplicationService,
};
use crate::contexts::agent_runtime::domain::MemoryType;
use crate::contexts::skill_evolution_evidence::domain::{
    ObservedSkillRevision, SkillAssociationKind,
};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolBinding, SkillToolCatalogContext, SkillToolCatalogMode, SkillToolCatalogPort,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_tool_catalog_with_code_intelligence(
    request: &GenerationProcessRequest,
    mcp: &dyn AgentMcpToolPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    plan_mode: bool,
    retrieval_available: bool,
    code_search_available: bool,
    code_intelligence_available: bool,
) -> Vec<ToolDefinition> {
    if plan_mode {
        let mut tools = plan_mode_tool_catalog();
        if retrieval_available {
            tools.push(recall_tool_definition());
        }
        if code_search_available {
            tools.push(search_code_tool_definition());
        }
        if code_intelligence_available {
            tools.extend(code_intelligence_tool_definitions());
        }
        // Plan mode is where clarification matters most -- the whole point of the mode is to
        // settle what the work is before doing it (`add-agent-user-question`).
        if request.interactive {
            tools.push(ask_user_question_tool_definition());
        }
        return tools;
    }
    let mut tools = tool_catalog();
    let project_path = request.session.folder.as_deref().unwrap_or_default();
    match mcp.catalog_entries(project_path) {
        Ok(mcp_tools) => tools.extend(mcp_tools),
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.mcp".to_string(),
                message: format!(
                    "Failed to resolve MCP-sourced tools; continuing with the fixed tool catalog only: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
        }
    }
    if retrieval_available {
        tools.push(recall_tool_definition());
    }
    if code_search_available {
        tools.push(search_code_tool_definition());
    }
    if code_intelligence_available {
        tools.extend(code_intelligence_tool_definitions());
    }
    if request.interactive {
        tools.push(ask_user_question_tool_definition());
    }
    tools
}

/// Everything the generation offers the model, assembled once before the round-trip loop: the
/// reviewed catalog above, then the delegation tool, then the eligible native tools, then the
/// endpoint Profile's capability veto. The three availability probes live here rather than in the
/// caller because nothing outside this assembly reads them.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_generation_tool_catalog(
    request: &GenerationProcessRequest,
    mcp: &dyn AgentMcpToolPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: &dyn AgentCodeIntelligencePort,
    native_tools: &NativeToolRegistry,
    utility_delegation: Option<&UtilityDelegationApplicationService>,
    plan_mode: bool,
    memory_read_allowed: bool,
) -> Vec<ToolDefinition> {
    // Never blocks, never errors (`AgentRetrievalPort::is_configured`'s own contract) — safe to
    // call unconditionally on every generation's catalog resolution, matching how `plan_mode`
    // itself is derived at the call site.
    //
    // Both conditions, because `recall` searches the same long-term memory pool the index draws
    // from. A configured retrieval index says the search *can* run; the snapshot says whether this
    // session may read memory at all. Offering the tool to a session that may not read would leave
    // a second door into the pool that suppressing the index alone does not close — and a
    // temporary session would have kept a working search over everything it was promised to
    // forget.
    let retrieval_available = retrieval.is_configured() && memory_read_allowed;
    let code_search_available = request
        .session
        .folder
        .as_deref()
        .and_then(|folder| {
            retrieval
                .code_retrieval()
                .map(|code| code.is_available(folder))
        })
        .unwrap_or(false);
    let code_intelligence_context = request
        .session
        .folder
        .as_deref()
        .map(AgentCodeIntelligenceContext::from_session_workspace);
    let code_intelligence_available = code_intelligence_context
        .as_ref()
        .is_some_and(|context| code_intelligence.is_available(context));
    let mut tools = resolve_tool_catalog_with_code_intelligence(
        request,
        mcp,
        logging,
        clock,
        plan_mode,
        retrieval_available,
        code_search_available,
        code_intelligence_available,
    );
    if utility_delegation.is_some() && !plan_mode {
        tools.push(delegate_utility_skill_tool_definition());
    }
    tools.extend(
        native_tools.eligible_tool_definitions(&ToolEligibilityContext {
            agent_id: request.agent.id.clone(),
            session_id: request.session.id.clone(),
            generation_id: request.operation_id.clone(),
            canonical_workspace: request.session.folder.as_deref().map(Into::into),
            execution_mode: if plan_mode {
                NativeToolExecutionMode::Plan
            } else {
                NativeToolExecutionMode::Execute
            },
            readiness: native_tools.readiness_snapshot(),
        }),
    );
    if request
        .endpoint_profile
        .as_ref()
        .is_some_and(|profile| profile.tool_calling_capability != "supported")
    {
        tools.clear();
    }
    tools
}

/// The skill-tool half of the generation's catalog. `None` means the catalog rejected the request:
/// that logs a warning and leaves the tools already resolved untouched, exactly as an MCP lookup
/// failure does above, because skill tools are additive on top of an already-usable catalog.
///
/// The resolved lease, generation counter and key map are returned whole rather than unpacked here
/// so the caller can keep them in the three bindings it already had. The lease is an `Arc` held for
/// the rest of the generation, and moving the three into one value would reorder their drops.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_generation_skill_tools(
    catalog: &dyn SkillToolCatalogPort,
    request: &GenerationProcessRequest,
    provider_config: &ApiProviderConfig,
    observed_skill_revisions: &[ObservedSkillRevision],
    existing_tools: &[ToolDefinition],
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    plan_mode: bool,
) -> Option<ResolvedSkillToolCatalog> {
    let loaded_roles = observed_skill_revisions
        .iter()
        .map(|observed| SkillToolBinding {
            skill_id: observed.skill_id.clone(),
            revision: observed.revision.clone(),
        })
        .collect::<Vec<_>>();
    let context = SkillToolCatalogContext::RoleGeneration {
        workspace_path: request.session.folder.clone(),
        loaded_roles,
        mode: if plan_mode {
            SkillToolCatalogMode::Plan
        } else {
            SkillToolCatalogMode::Execute
        },
    };
    let existing_names = existing_tools.iter().map(|tool| tool.name.clone());
    match resolve_skill_tool_catalog(
        catalog,
        &context,
        existing_names,
        &provider_config.interface_format,
    ) {
        Ok(resolved) => Some(resolved),
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skill-tools".to_string(),
                message: format!("Skill tool catalog rejected: {}", error.code()),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    }
}

/// This generation's personalization: the answer it was planned around, and the boundary it can
/// propose back through.
///
/// Carried together because neither is usable alone. A snapshot without the boundary cannot act on
/// what it resolved, and the boundary without the snapshot has no eligible set to judge a proposal
/// against — which is exactly the pair a second, independently-resolved read would break.
#[derive(Clone, Copy)]
pub(super) struct GenerationPersonalization<'a> {
    pub(super) snapshot: &'a AgentPersonalizationSnapshot,
    pub(super) port: &'a dyn AgentPersonalizationSnapshotPort,
}

/// Records what the model asked to remember, as a proposal.
///
/// The tool keeps its name and its place in the catalog, and stops writing a memory. A model
/// deciding on its own what the user will still be told six months from now is the behaviour this
/// replaces: what it produces now is a queue entry, and a person decides.
///
/// Every gate comes from this generation's snapshot rather than a fresh read. A second read could
/// disagree with the one the prompt was built from, which would allow a tool call against a policy
/// the model was never told about.
pub(super) fn propose_remembered_memory(
    input: &serde_json::Value,
    personalization: GenerationPersonalization<'_>,
    request: &GenerationProcessRequest,
) -> ToolExecutionOutcome {
    let content = input
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if content.is_empty() {
        return ToolExecutionOutcome {
            output: "No content was provided to remember.".to_string(),
            is_error: true,
        };
    }
    // Two separate answers, and both must hold. A temporary session forbids proposing one even
    // where saving would otherwise have been permitted.
    if !personalization.snapshot.memory.explicit_save
        || !personalization.snapshot.memory.candidate_creation
    {
        return ToolExecutionOutcome {
            output: "Memory is disabled; nothing was remembered.".to_string(),
            is_error: true,
        };
    }
    let name = input
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let description = input
        .get("description")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let memory_type = input
        .get("type")
        .and_then(serde_json::Value::as_str)
        .and_then(MemoryType::parse);
    let submission = AgentCandidateSubmission {
        proposals: vec![AgentMemoryProposal::Create {
            // A name is how a person finds the proposal in a queue, so an unnamed one gets the
            // first line of what it holds rather than a placeholder that describes nothing.
            name: if name.is_empty() {
                content.lines().next().unwrap_or(content).to_string()
            } else {
                name.to_string()
            },
            description: if description.is_empty() {
                content.lines().next().unwrap_or(content).to_string()
            } else {
                description.to_string()
            },
            memory_type,
            content: content.to_string(),
        }],
        origin: AgentProposalOrigin::ModelTool,
        agent_id: request.agent.id.clone(),
        session_id: request.session.id.clone(),
        folder: request.session.folder.clone(),
        eligible: personalization.snapshot.memory.eligible.clone(),
    };
    match personalization.port.propose_memories(submission) {
        // The model is told what actually happened. Reporting "Saved." for something awaiting
        // review would have it act, later in the same session, as though the fact were settled.
        Ok(outcome) if outcome.accepted > 0 => ToolExecutionOutcome {
            output: "Proposed for review. It is not in memory until the user approves it."
                .to_string(),
            is_error: false,
        },
        Ok(_) => ToolExecutionOutcome {
            output: "The proposal was refused and nothing was remembered.".to_string(),
            is_error: true,
        },
        Err(error) => ToolExecutionOutcome {
            output: format!("Failed to propose a memory: {error}"),
            is_error: true,
        },
    }
}

/// Takes this generation's one personalization snapshot, and reports it if it was lost.
///
/// One call, at the start of a generation, and the answer is reused for the whole of it. A policy
/// edit made while the turn runs reaches the next turn rather than rewriting a prompt already
/// assembled under the previous answer — which is what stops a turn whose memory index and whose
/// selected bodies were resolved under two different policies.
///
/// A generation that silently lost its personalization is indistinguishable, from the outside,
/// from a user who configured none, so the loss is recorded. The reason is a stable code — never a
/// path, never a store error, never instruction text — because this is a log line.
pub(super) fn resolve_generation_personalization(
    personalization: &dyn AgentPersonalizationSnapshotPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> AgentPersonalizationSnapshot {
    let snapshot = personalization.snapshot(GenerationPersonalizationContext {
        agent_id: request.agent.id.clone(),
        session_id: request.session.id.clone(),
        folder: request.session.folder.clone(),
    });
    if let Some(reason) = snapshot.memory.blocked_reason.as_deref() {
        let _ = logging.record(AgentLog {
            level: AgentLogLevel::Warn,
            category: "session.runtime.api.personalization".to_string(),
            message: format!(
                "Personalization unavailable ({reason}); continuing without custom instructions or memory."
            ),
            agent_id: Some(request.agent.id.clone()),
            session_id: Some(request.session.id.clone()),
            operation_id: Some(request.operation_id.clone()),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: clock.now(),
        });
    }
    snapshot
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_system_prompt_with_settings(
    agent_id: &str,
    core_instructions: &dyn AgentCoreInstructionsPort,
    snapshot: &AgentPersonalizationSnapshot,
    skills: &dyn AgentSkillPort,
    personalization: &dyn AgentPersonalizationSnapshotPort,
    selection: &dyn AgentMemorySelectionPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    observed_skill_revisions: &mut Vec<ObservedSkillRevision>,
) -> Option<String> {
    // Already merged and ordered by policy. This function places it; it no longer decides whether
    // instructions apply, which layer authored them, or in what order they combine.
    let custom_instructions_section = snapshot.instruction_block.clone();
    let core_section = match core_instructions.instructions_for(agent_id) {
        Ok(Some(core)) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Debug,
                category: "session.runtime.api.prompt".to_string(),
                message: format!("Applied core instructions version {}.", core.version),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            Some(core.markdown)
        }
        Ok(None) => None,
        Err(_) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.prompt".to_string(),
                message:
                    "Failed to resolve core instructions; continuing with optional prompt sections."
                        .to_string(),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    let skill_section = match skills
        .bound_skill_prompts(agent_id, request.session.folder.as_deref())
    {
        Ok(prompts) if prompts.is_empty() => None,
        Ok(prompts) => {
            let observed_at = clock.now();
            observed_skill_revisions.extend(prompts.iter().map(|prompt| ObservedSkillRevision {
                skill_id: prompt.id.clone(),
                revision: prompt.revision.clone(),
                association_kind: SkillAssociationKind::Injected,
                observed_at: observed_at.clone(),
            }));
            format_system_prompt(&prompts, logging, clock, request)
        }
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skills".to_string(),
                message: format!(
                    "Failed to resolve bound Skills; continuing without them in the system prompt: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    // The eligible set is what the snapshot found; nothing here can reach outside it. Policy, the
    // session mode, the runtime capability and migration health were all applied before this
    // function saw a single record, which is why there is no toggle left to check here beyond what
    // the snapshot already decided.
    let eligible: Vec<AgentMemory> = snapshot
        .memory
        .eligible
        .iter()
        .map(memory_from_ref)
        .collect();
    let (memory_section, memory_bodies_section) = match snapshot.memory.delivery {
        // Nothing is fetched rather than fetched and discarded: a runtime that cannot take an index
        // has no use for one, and neither does a session that may not read.
        AgentMemoryDelivery::None => (None, None),
        AgentMemoryDelivery::IndexOnly => (format_memory_section(&eligible), None),
        AgentMemoryDelivery::IndexWithSelectedBodies => (
            format_memory_section(&eligible),
            select_memory_bodies(
                &snapshot.memory.eligible,
                &eligible,
                personalization,
                selection,
                logging,
                clock,
                request,
            ),
        ),
    };
    // Changes on every `todo_write` (`add-agent-task-list` D2), so it is the most volatile section
    // of all and sits last.
    let task_list_section = task_list_prompt_section(&request.session.id);
    // Stable content first, volatile last. A prefix cache is a prefix, so the sections that change
    // most often sit at the tail where they invalidate the least. The memory index reflects the
    // pool and the bodies reflect one generation's judgment about it, so the bodies follow the
    // index; the task list changes more often than either and follows both.
    let sections: Vec<String> = [
        core_section,
        custom_instructions_section,
        skill_section,
        memory_section,
        memory_bodies_section,
        task_list_section,
    ]
    .into_iter()
    .flatten()
    .collect();
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

/// Runs the relevance selection for one generation and formats the chosen bodies.
///
/// Runs once here, at generation start, rather than per provider round trip. That is forced by
/// two things at once: memory content must never enter the turns list compaction manipulates, so
/// bodies have to live in the system prompt; and a system prompt that changed every round trip
/// would invalidate the provider prefix cache on every round trip inside a tool loop.
///
/// Any failure degrades to index-only injection. Selection is an enhancement — its loss costs
/// relevance, never the generation, and the index alone still tells the model what exists.
#[allow(clippy::too_many_arguments)]
fn select_memory_bodies(
    refs: &[AgentMemoryRef],
    memories: &[AgentMemory],
    personalization: &dyn AgentPersonalizationSnapshotPort,
    selection: &dyn AgentMemorySelectionPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> Option<String> {
    if memories.is_empty() {
        return None;
    }
    // Excluded before the call, not after: filtering afterwards would spend the bounded selection
    // budget on memories this session has already been shown and the caller is about to discard.
    let candidates = unsurfaced_candidates(&request.session.id, memories);
    if candidates.is_empty() {
        return None;
    }
    let selected_names = match selection.select(&request.effective_prompt, &candidates) {
        Ok(names) => names,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Memory relevance selection failed; continuing with the index alone: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return None;
        }
    };
    // Follows the selector's own order so its ranking survives into the prompt. A name the selector
    // returned that is not among the candidates is dropped rather than looked up elsewhere — that
    // is the only place selection could otherwise reach past what policy allowed.
    let selected = selected_names
        .iter()
        .filter_map(|name| {
            candidates
                .iter()
                .find(|memory| &memory.name == name)
                .cloned()
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return None;
    }
    // Fetched at the revisions the snapshot pinned. A memory edited since this generation began is
    // absent rather than silently newer, so the body in the prompt is the body the index described.
    let pinned: Vec<AgentMemoryRef> = selected
        .iter()
        .filter_map(|memory| refs.iter().find(|entry| entry.id == memory.id).cloned())
        .collect();
    let bodies = match personalization.pinned_bodies(&pinned) {
        Ok(bodies) => bodies,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Selected memory bodies could not be read; continuing with the index alone: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return None;
        }
    };
    // Marked only for what actually reached the prompt. Marking the selection instead would hide a
    // memory from later turns that this one never showed.
    let surfaced: Vec<AgentMemory> = pinned
        .iter()
        .filter_map(|entry| {
            let body = bodies.iter().find(|body| body.id == entry.id)?;
            Some(AgentMemory {
                content: body.content.clone(),
                ..memory_from_ref(entry)
            })
        })
        .collect();
    mark_surfaced(&request.session.id, &surfaced);
    crate::contexts::agent_runtime::application::format_memory_bodies(
        &surfaced,
        std::time::SystemTime::now(),
    )
}

/// One eligible ref in the shape the index, the selector and the surfaced tracker already speak.
///
/// The body is deliberately empty: none of those three reads it, and filling it would mean loading
/// every eligible memory to build an index that names them.
fn memory_from_ref(entry: &AgentMemoryRef) -> AgentMemory {
    AgentMemory {
        id: entry.id.clone(),
        agent_id: String::new(),
        folder: None,
        name: entry.name.clone(),
        description: entry.description.clone(),
        memory_type: entry.memory_type,
        content: String::new(),
        source: MemorySource::Automatic,
        created_at: String::new(),
        // Carried through, and the reason the staleness caveat and the already-surfaced exclusion
        // work at all: both key on it, and a `None` here silently disables both.
        modified_at: entry.updated_at,
    }
}

pub(super) fn format_system_prompt(
    prompts: &[BoundSkillPrompt],
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> Option<String> {
    let mut used = 0usize;
    let mut sections = Vec::new();
    for prompt in prompts {
        let section = format!("## {}\n{}", prompt.name, prompt.body);
        let length = section.chars().count();
        let reason = if length > SKILL_PER_ITEM_CHARACTER_BUDGET {
            Some("per-Skill 8,000-character budget")
        } else if used.saturating_add(length) > SKILL_AGGREGATE_CHARACTER_BUDGET {
            Some("aggregate 16,000-character budget")
        } else {
            None
        };
        if let Some(reason) = reason {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skills".to_string(),
                message: format!(
                    "Skipped Skill {} because it exceeds the {reason}",
                    prompt.id
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            continue;
        }
        used += length;
        sections.push(section);
    }
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

/// Thin delegate to `application::format_memory_index` (the formatting rule lives there so the
/// CLI-wrapped agents' send path can share it without `application` depending on
/// `infrastructure` — mirrors `format_custom_instructions_section`'s existing delegation shape).
///
/// Binds OnePiece's bounds here rather than at the call site: this surface is the system prompt,
/// and the CLI surface's far tighter bounds must never be reachable from it by accident.
pub(super) fn format_memory_section(memories: &[AgentMemory]) -> Option<String> {
    crate::contexts::agent_runtime::application::format_memory_index(
        memories,
        crate::contexts::agent_runtime::application::ONEPIECE_MEMORY_INDEX_BOUNDS,
    )
}
