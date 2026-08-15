use super::{
    ASK_USER_QUESTION_TOOL_NAME, EDIT_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME, FILE_TOOL_NAME,
    FIND_DEFINITION_TOOL_NAME, FIND_REFERENCES_TOOL_NAME, GET_DIAGNOSTICS_TOOL_NAME,
    GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME, GREP_TOOL_NAME, LIST_SKILLS_TOOL_NAME,
    LOAD_SKILL_TOOL_NAME, MCP_TOOL_NAME_PREFIX, READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME,
    REMEMBER_TOOL_NAME, SEARCH_CODE_TOOL_NAME, SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME,
    SHELL_TOOL_NAME, TODO_WRITE_TOOL_NAME,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistingToolHandler {
    Shell,
    ShellOutput,
    ShellKill,
    TodoWrite,
    AskUserQuestion,
    File,
    Grep,
    Glob,
    Edit,
    Remember,
    Recall,
    SearchCode,
    SkillRead,
    CodeIntelligence,
    Mcp,
    /// Resolved so the catalog and this registry stay in agreement, but never dispatched
    /// through `execute_tool_call_impl`: like a question, leaving plan mode needs the event
    /// sink and the blocked-call channel, so the tool loop handles it before reaching here.
    ExitPlanMode,
}

pub(crate) struct ExistingToolHandlerRegistry;

impl ExistingToolHandlerRegistry {
    pub(crate) fn resolve(name: &str) -> Option<ExistingToolHandler> {
        match name {
            SHELL_TOOL_NAME => Some(ExistingToolHandler::Shell),
            SHELL_OUTPUT_TOOL_NAME => Some(ExistingToolHandler::ShellOutput),
            SHELL_KILL_TOOL_NAME => Some(ExistingToolHandler::ShellKill),
            TODO_WRITE_TOOL_NAME => Some(ExistingToolHandler::TodoWrite),
            ASK_USER_QUESTION_TOOL_NAME => Some(ExistingToolHandler::AskUserQuestion),
            EXIT_PLAN_MODE_TOOL_NAME => Some(ExistingToolHandler::ExitPlanMode),
            FILE_TOOL_NAME => Some(ExistingToolHandler::File),
            GREP_TOOL_NAME => Some(ExistingToolHandler::Grep),
            GLOB_TOOL_NAME => Some(ExistingToolHandler::Glob),
            EDIT_TOOL_NAME => Some(ExistingToolHandler::Edit),
            REMEMBER_TOOL_NAME => Some(ExistingToolHandler::Remember),
            RECALL_TOOL_NAME => Some(ExistingToolHandler::Recall),
            SEARCH_CODE_TOOL_NAME => Some(ExistingToolHandler::SearchCode),
            LIST_SKILLS_TOOL_NAME | LOAD_SKILL_TOOL_NAME | READ_SKILL_RESOURCE_TOOL_NAME => {
                Some(ExistingToolHandler::SkillRead)
            }
            FIND_DEFINITION_TOOL_NAME
            | FIND_REFERENCES_TOOL_NAME
            | GET_HOVER_TOOL_NAME
            | GET_DIAGNOSTICS_TOOL_NAME => Some(ExistingToolHandler::CodeIntelligence),
            _ if name.starts_with(MCP_TOOL_NAME_PREFIX) => Some(ExistingToolHandler::Mcp),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        code_intelligence_tool_definitions, plan_mode_tool_catalog, recall_tool_definition,
        search_code_tool_definition, tool_catalog,
    };

    #[test]
    fn every_existing_catalog_definition_resolves_to_a_registered_handler() {
        let mut definitions = tool_catalog();
        definitions.extend(plan_mode_tool_catalog());
        definitions.push(recall_tool_definition());
        definitions.push(search_code_tool_definition());
        definitions.extend(code_intelligence_tool_definitions());

        for definition in definitions {
            assert!(
                ExistingToolHandlerRegistry::resolve(&definition.name).is_some(),
                "missing handler registration for {}",
                definition.name
            );
        }
    }

    #[test]
    fn mcp_names_resolve_without_creating_inventory_specific_schemas() {
        assert_eq!(
            ExistingToolHandlerRegistry::resolve("mcp__fixture__search"),
            Some(ExistingToolHandler::Mcp)
        );
        assert_eq!(ExistingToolHandlerRegistry::resolve("unknown"), None);
    }
}
