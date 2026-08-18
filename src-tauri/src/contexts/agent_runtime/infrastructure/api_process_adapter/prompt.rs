//! Tool catalog resolution, system prompt assembly, personalization, and memory sections.

use super::super::memory_surfaced::{mark_surfaced, unsurfaced_candidates};
use super::super::skill_tool_catalog_adapter::{
    resolve_skill_tool_catalog, ResolvedSkillToolCatalog,
};
use super::super::tools::task_list_prompt_section;
use super::{SKILL_AGGREGATE_CHARACTER_BUDGET, SKILL_PER_ITEM_CHARACTER_BUDGET};
use crate::contexts::agent_runtime::application::{
    ask_user_question_tool_definition, code_intelligence_tool_definitions,
    delegate_utility_skill_tool_definition, plan_mode_tool_catalog, recall_tool_definition,
    search_code_tool_definition, tool_catalog, AgentClockPort, AgentCodeIntelligenceContext,
    AgentCodeIntelligencePort, AgentCoreInstructionsPort, AgentLog, AgentLogLevel,
    AgentLoggingPort, AgentMcpToolPort, AgentMemory, AgentMemoryPort, AgentMemorySelectionPort,
    AgentPersonalizationPort, AgentRetrievalPort, AgentSkillPort, ApiProviderConfig,
    BoundSkillPrompt, GenerationProcessRequest, NativeToolExecutionMode, NativeToolRegistry,
    PersonalizationSettings, ToolDefinition, ToolEligibilityContext,
    UtilityDelegationApplicationService,
};
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
) -> Vec<ToolDefinition> {
    // Never blocks, never errors (`AgentRetrievalPort::is_configured`'s own contract) — safe to
    // call unconditionally on every generation's catalog resolution, matching how `plan_mode`
    // itself is derived at the call site.
    let retrieval_available = retrieval.is_configured();
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

/// Resolves the agent's bound, enabled Skills (`add-agent-skill-support`) and stored memories
/// scoped to `(agent_id, request.session.folder)` (`add-agent-cross-session-memory`) into one
/// system-prompt string, or `None` if both are empty. Neither source can fail the generation on
/// lookup error — each logs its own warning and falls back to contributing nothing, matching
/// context compaction's own established best-effort-enhancement philosophy (design.md Decision 3
/// in `add-agent-skill-support`).
/// Fetches host-level personalization settings once, degrading to
/// `PersonalizationSettings::safe_fallback()` and a logged warning on lookup failure — shared by
/// every call site that needs a personalization flag (`add-personalization-settings`), matching
/// this function's neighbors' own established lookup-failure philosophy.
pub(super) fn resolve_personalization_settings(
    personalization: &dyn AgentPersonalizationPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> PersonalizationSettings {
    match personalization.settings() {
        Ok(settings) => settings,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.personalization".to_string(),
                message: format!(
                    "Failed to resolve personalization settings; continuing with safe defaults: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            PersonalizationSettings::safe_fallback()
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_system_prompt_with_settings(
    agent_id: &str,
    core_instructions: &dyn AgentCoreInstructionsPort,
    personalization_settings: &PersonalizationSettings,
    skills: &dyn AgentSkillPort,
    memories: &dyn AgentMemoryPort,
    selection: &dyn AgentMemorySelectionPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    observed_skill_revisions: &mut Vec<ObservedSkillRevision>,
) -> Option<String> {
    let custom_instructions_section = format_custom_instructions_section(personalization_settings);
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
    let (memory_section, memory_bodies_section) = if !personalization_settings.memory_enabled {
        // Memory master switch off (`add-personalization-settings` D4) — skip the lookup
        // entirely rather than fetching and discarding, matching design.md D8's "no wasted work
        // when a feature is off" intent. No selection call is made either.
        (None, None)
    } else {
        match memories.list_all() {
            Ok(memories) => (
                format_memory_section(&memories),
                select_memory_bodies(&memories, selection, logging, clock, request),
            ),
            Err(error) => {
                let _ = logging.record(AgentLog {
                    level: AgentLogLevel::Warn,
                    category: "session.runtime.api.memory".to_string(),
                    message: format!(
                        "Failed to resolve stored memories; continuing without them in the system prompt: {error}"
                    ),
                    agent_id: Some(request.agent.id.clone()),
                    session_id: Some(request.session.id.clone()),
                    operation_id: Some(request.operation_id.clone()),
                    run_id: None,
                    trace_id: None,
                    span_id: None,
                    occurred_at: clock.now(),
                });
                (None, None)
            }
        }
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
fn select_memory_bodies(
    memories: &[AgentMemory],
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
    // Follows the selector's own order so its ranking survives into the prompt.
    let selected = selected_names
        .iter()
        .filter_map(|name| {
            candidates
                .iter()
                .find(|memory| &memory.name == name)
                .cloned()
        })
        .collect::<Vec<_>>();
    mark_surfaced(&request.session.id, &selected);
    crate::contexts::agent_runtime::application::format_memory_bodies(
        &selected,
        std::time::SystemTime::now(),
    )
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

/// Formats enabled, non-empty custom instructions into one `## Custom Instructions` section,
/// response style before about-you within it (`add-personalization-settings` design.md D3 — style
/// is a cross-cutting constraint on every response, about-you is background fact, so style gets
/// the higher-priority earlier position). Returns `None` when disabled or both fields are empty,
/// omitting either sub-heading individually when only one field is populated.
/// Thin delegate to `PersonalizationSettings::custom_instructions_block` (moved to `application`
/// in `add-cli-custom-instructions-injection` so the CLI-wrapped agents' send path can share the
/// identical formatting rule without `application` depending on `infrastructure`). Kept as a free
/// function here, rather than updating every call site to the method form, so this file's existing
/// `format_custom_instructions_section_*` tests need no changes.
pub(super) fn format_custom_instructions_section(
    settings: &PersonalizationSettings,
) -> Option<String> {
    settings.custom_instructions_block()
}
