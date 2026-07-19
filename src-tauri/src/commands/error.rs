use crate::contexts::agent_runtime::api::AgentRuntimeApplicationError;
use crate::contexts::communications::api::CommunicationsApplicationError;
use crate::contexts::desktop::api::{DesktopSettingsError, FloatingAssistantError};
use crate::contexts::operations::application::ApplicationError;
use crate::contexts::sessions::api::SessionsError;
use crate::contexts::tooling::cli::api::CliError;
use crate::contexts::tooling::cli_parameters::CliParametersError;
use crate::contexts::tooling::extensions::api::ExtensionError;
use crate::contexts::tooling::mcp::api::McpError;
use crate::contexts::tooling::plugin_integrations::api::PluginIntegrationError;
use crate::contexts::tooling::prompt_hooks::api::PromptHookError;
use crate::contexts::tooling::sdk::api::SdkError;
use crate::contexts::tooling::skills::api::{SkillDomainError, SkillError};
use crate::contexts::workspaces::api::WorkspaceError;
use crate::platform::error::InfrastructureError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandErrorCategory {
    Validation,
    NotFound,
    Conflict,
    Unsupported,
    Unavailable,
    Infrastructure,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandError {
    category: CommandErrorCategory,
    message: String,
}

impl CommandError {
    #[cfg(test)]
    pub(crate) fn category(&self) -> CommandErrorCategory {
        self.category
    }

    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn validation(message: impl Into<String>) -> Self {
        Self {
            category: CommandErrorCategory::Validation,
            message: format!("validation error: {}", message.into()),
        }
    }

    pub(crate) fn storage(message: impl Into<String>) -> Self {
        Self {
            category: CommandErrorCategory::Infrastructure,
            message: format!("storage error: {}", message.into()),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CommandError {}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.message)
    }
}

pub(crate) fn map_command_error(error: impl Into<CommandError>) -> CommandError {
    error.into()
}

impl From<ApplicationError> for CommandError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::NotFound(message) => Self {
                category: CommandErrorCategory::NotFound,
                message,
            },
            ApplicationError::Infrastructure { message, .. } => Self {
                category: CommandErrorCategory::Infrastructure,
                message,
            },
            ApplicationError::Internal(message) => Self {
                category: CommandErrorCategory::Internal,
                message,
            },
        }
    }
}

impl From<InfrastructureError> for CommandError {
    fn from(error: InfrastructureError) -> Self {
        Self {
            category: CommandErrorCategory::Infrastructure,
            message: error.command_safe_message().to_string(),
        }
    }
}

impl From<CommunicationsApplicationError> for CommandError {
    fn from(error: CommunicationsApplicationError) -> Self {
        let category = match &error {
            CommunicationsApplicationError::Domain(_) => CommandErrorCategory::Validation,
            CommunicationsApplicationError::Failure { .. } => CommandErrorCategory::Infrastructure,
        };
        Self {
            category,
            message: error.safe_code().to_string(),
        }
    }
}

impl From<CliParametersError> for CommandError {
    fn from(error: CliParametersError) -> Self {
        match error {
            CliParametersError::Validation(message) => Self::validation(message),
            CliParametersError::Repository(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
        }
    }
}

impl From<AgentRuntimeApplicationError> for CommandError {
    fn from(error: AgentRuntimeApplicationError) -> Self {
        match error {
            AgentRuntimeApplicationError::Domain(error) => Self::validation(error.to_string()),
            AgentRuntimeApplicationError::Validation(message) => Self::validation(message),
            AgentRuntimeApplicationError::AgentNotFound(agent_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("agent not found: {agent_id}"),
            },
            AgentRuntimeApplicationError::SessionNotFound(session_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("session not found: {session_id}"),
            },
            AgentRuntimeApplicationError::MessageNotFound(message_id) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: Message not found: {message_id}"),
            },
            AgentRuntimeApplicationError::NoActiveAgent => Self {
                category: CommandErrorCategory::Validation,
                message: "no active agent selected".to_string(),
            },
            AgentRuntimeApplicationError::AgentUnavailable(message) => Self {
                category: CommandErrorCategory::Unavailable,
                message: format!("agent is unavailable: {message}"),
            },
            AgentRuntimeApplicationError::UnsupportedInteractionMode(mode) => Self {
                category: CommandErrorCategory::Unsupported,
                message: format!("unsupported interaction mode: {mode}"),
            },
            AgentRuntimeApplicationError::GenerationConflict(session_id) => Self {
                category: CommandErrorCategory::Conflict,
                message: format!(
                    "validation error: A generation is already active for session {session_id}."
                ),
            },
            AgentRuntimeApplicationError::Registry(message)
            | AgentRuntimeApplicationError::Workflow(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            AgentRuntimeApplicationError::Process(message) => Self {
                category: CommandErrorCategory::Unavailable,
                message: format!("launch failed: {message}"),
            },
            AgentRuntimeApplicationError::Session(message)
            | AgentRuntimeApplicationError::CliProfile(message)
            | AgentRuntimeApplicationError::Prompt(message)
            | AgentRuntimeApplicationError::Operation(message)
            | AgentRuntimeApplicationError::Logging(message)
            | AgentRuntimeApplicationError::Event(message)
            | AgentRuntimeApplicationError::Generation(message) => Self::storage(message),
        }
    }
}

impl From<DesktopSettingsError> for CommandError {
    fn from(error: DesktopSettingsError) -> Self {
        match error {
            DesktopSettingsError::Domain(error) => Self::validation(error.to_string()),
            DesktopSettingsError::Repository(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "database error: ",
            ),
            DesktopSettingsError::NetworkProxy(message) => command_error_with_default(
                CommandErrorCategory::Unavailable,
                message,
                "launch failed: ",
            ),
            DesktopSettingsError::LogDirectory(message)
            | DesktopSettingsError::Startup(message)
            | DesktopSettingsError::Directory(message)
            | DesktopSettingsError::ClientLogging(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "storage error: ",
            ),
        }
    }
}

impl From<FloatingAssistantError> for CommandError {
    fn from(error: FloatingAssistantError) -> Self {
        match error {
            FloatingAssistantError::Domain(error) => Self::validation(error.to_string()),
            FloatingAssistantError::Repository(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "database error: ",
            ),
            FloatingAssistantError::Window(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "storage error: ",
            ),
        }
    }
}

impl From<WorkspaceError> for CommandError {
    fn from(error: WorkspaceError) -> Self {
        match error {
            WorkspaceError::Domain(error) => Self::validation(error.to_string()),
            WorkspaceError::Validation(message) => Self::validation(message),
            WorkspaceError::Repository(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "database error: ",
            ),
            WorkspaceError::Selection(message) | WorkspaceError::Filesystem(message) => {
                command_error_with_default(
                    CommandErrorCategory::Infrastructure,
                    message,
                    "storage error: ",
                )
            }
            WorkspaceError::Storage(message) => command_error_with_default(
                CommandErrorCategory::Infrastructure,
                message,
                "storage error: ",
            ),
            WorkspaceError::LaunchFailed(message) => command_error_with_default(
                CommandErrorCategory::Unavailable,
                message,
                "launch failed: ",
            ),
            WorkspaceError::SessionNotFound(session_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("session not found: {session_id}"),
            },
        }
    }
}

impl From<SessionsError> for CommandError {
    fn from(error: SessionsError) -> Self {
        match error {
            SessionsError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            SessionsError::Validation(message) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {message}"),
            },
            SessionsError::AgentNotFound(agent_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("agent not found: {agent_id}"),
            },
            SessionsError::UnsupportedInteractionMode(mode) => Self {
                category: CommandErrorCategory::Unsupported,
                message: format!("unsupported interaction mode: {mode}"),
            },
            SessionsError::SessionNotFound(session_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("session not found: {session_id}"),
            },
            SessionsError::MessageNotFound(message_id) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: Message not found: {message_id}"),
            },
            SessionsError::CategoryNotFound(category_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("validation error: Category not found: {category_id}"),
            },
            SessionsError::CategoryNameConflict(_) => Self {
                category: CommandErrorCategory::Conflict,
                message: "validation error: Category name already exists.".to_string(),
            },
            SessionsError::Repository(message) | SessionsError::Transaction(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            SessionsError::WorkspaceLaunch(message) | SessionsError::RuntimeLaunch(message) => {
                Self {
                    category: CommandErrorCategory::Unavailable,
                    message: format!("launch failed: {message}"),
                }
            }
            SessionsError::FileContent(message)
            | SessionsError::Operation(message)
            | SessionsError::Logging(message)
            | SessionsError::Serialization(message)
            | SessionsError::Workspace(message)
            | SessionsError::Runtime(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

fn command_error_with_default(
    fallback_category: CommandErrorCategory,
    message: String,
    default_prefix: &str,
) -> CommandError {
    let category = if message.starts_with("validation error: ") {
        CommandErrorCategory::Validation
    } else if message.starts_with("launch failed: ") {
        CommandErrorCategory::Unavailable
    } else {
        fallback_category
    };
    let message = if command_error_has_prefix(&message) {
        message
    } else {
        format!("{default_prefix}{message}")
    };
    CommandError { category, message }
}

fn command_error_has_prefix(message: &str) -> bool {
    [
        "validation error: ",
        "database error: ",
        "storage error: ",
        "launch failed: ",
    ]
    .iter()
    .any(|prefix| message.starts_with(prefix))
}

impl From<McpError> for CommandError {
    fn from(error: McpError) -> Self {
        match error {
            McpError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            McpError::ServerNotFound(name) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("MCP server not found: {name}"),
            },
            McpError::Validation(message) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {message}"),
            },
            McpError::Database(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            McpError::Storage(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

impl From<CliError> for CommandError {
    fn from(error: CliError) -> Self {
        match error {
            CliError::Validation(message) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {message}"),
            },
            CliError::Database(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            CliError::Storage(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
            CliError::Detection(message) | CliError::Package(message) => Self {
                category: CommandErrorCategory::Internal,
                message,
            },
            CliError::Operation(message) | CliError::Logging(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message,
            },
            CliError::Internal(message) => Self {
                category: CommandErrorCategory::Internal,
                message,
            },
        }
    }
}

impl From<SdkError> for CommandError {
    fn from(error: SdkError) -> Self {
        match error {
            SdkError::Validation(message) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {message}"),
            },
            SdkError::Repository(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
            SdkError::Package(message) => Self {
                category: CommandErrorCategory::Internal,
                message,
            },
            SdkError::Operation(message) | SdkError::Logging(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

impl From<ExtensionError> for CommandError {
    fn from(error: ExtensionError) -> Self {
        match error {
            ExtensionError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            ExtensionError::ConcurrentMutation(message) => Self {
                category: CommandErrorCategory::Conflict,
                message: format!("validation error: {message}"),
            },
            ExtensionError::Repository(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            ExtensionError::Runtime(message) => Self {
                category: CommandErrorCategory::Unavailable,
                message: format!("launch failed: {message}"),
            },
            ExtensionError::Installation(message)
            | ExtensionError::Operation(message)
            | ExtensionError::Logging(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

impl From<PluginIntegrationError> for CommandError {
    fn from(error: PluginIntegrationError) -> Self {
        match error {
            PluginIntegrationError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            PluginIntegrationError::Logging(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

impl From<SkillError> for CommandError {
    fn from(error: SkillError) -> Self {
        match error {
            SkillError::Domain(SkillDomainError::UnknownAgent(agent_id)) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("agent not found: {agent_id}"),
            },
            SkillError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            SkillError::Validation(message) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {message}"),
            },
            SkillError::NotFound(skill_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("validation error: Skill not found: {skill_id}"),
            },
            SkillError::Conflict(skill_id) => Self {
                category: CommandErrorCategory::Conflict,
                message: format!("validation error: Skill already exists: {skill_id}"),
            },
            SkillError::Repository(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
            SkillError::Filesystem(message)
            | SkillError::Selection(message)
            | SkillError::Logging(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("storage error: {message}"),
            },
        }
    }
}

impl From<PromptHookError> for CommandError {
    fn from(error: PromptHookError) -> Self {
        match error {
            PromptHookError::Domain(error) => Self {
                category: CommandErrorCategory::Validation,
                message: format!("validation error: {error}"),
            },
            PromptHookError::NotFound(hook_id) => Self {
                category: CommandErrorCategory::NotFound,
                message: format!("validation error: Prompt Hook not found: {hook_id}"),
            },
            PromptHookError::Conflict(hook_id) => Self {
                category: CommandErrorCategory::Conflict,
                message: format!("validation error: Prompt Hook already exists: {hook_id}"),
            },
            PromptHookError::Repository(message) => Self {
                category: CommandErrorCategory::Infrastructure,
                message: format!("database error: {message}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::prompt_hooks::domain::PromptHookDomainError;

    #[test]
    fn application_errors_keep_category_and_safe_message() {
        let error = map_command_error(ApplicationError::NotFound(
            "operation not found: operation-1".to_string(),
        ));

        assert_eq!(error.category(), CommandErrorCategory::NotFound);
        assert_eq!(error.message(), "operation not found: operation-1");
        assert_eq!(
            serde_json::to_value(error).expect("serialize command error"),
            serde_json::json!("operation not found: operation-1")
        );
    }

    #[test]
    fn infrastructure_diagnostics_do_not_cross_command_boundary() {
        let error = map_command_error(InfrastructureError::Database(
            "sqlite path C:\\private\\vanehub.sqlite failed".to_string(),
        ));

        assert_eq!(error.category(), CommandErrorCategory::Infrastructure);
        assert_eq!(error.message(), "The local database operation failed.");
        assert!(!error.to_string().contains("C:\\private"));
    }

    #[test]
    fn mcp_application_errors_preserve_legacy_tauri_strings() {
        let validation = map_command_error(McpError::Validation("invalid fixture".to_string()));
        let missing = map_command_error(McpError::ServerNotFound("missing-tools".to_string()));
        let database = map_command_error(McpError::Database("fixture failure".to_string()));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!("validation error: invalid fixture")
        );
        assert_eq!(
            serde_json::to_value(missing).expect("missing"),
            serde_json::json!("MCP server not found: missing-tools")
        );
        assert_eq!(
            serde_json::to_value(database).expect("database"),
            serde_json::json!("database error: fixture failure")
        );
    }

    #[test]
    fn cli_application_errors_preserve_legacy_tauri_strings() {
        let validation = map_command_error(CliError::Validation("invalid fixture".to_string()));
        let database = map_command_error(CliError::Database("fixture failure".to_string()));
        let storage = map_command_error(CliError::Storage("lock unavailable".to_string()));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!("validation error: invalid fixture")
        );
        assert_eq!(
            serde_json::to_value(database).expect("database"),
            serde_json::json!("database error: fixture failure")
        );
        assert_eq!(
            serde_json::to_value(storage).expect("storage"),
            serde_json::json!("storage error: lock unavailable")
        );
    }

    #[test]
    fn sdk_application_errors_preserve_legacy_tauri_strings() {
        let validation = map_command_error(SdkError::Validation("invalid fixture".to_string()));
        let repository = map_command_error(SdkError::Repository("lock unavailable".to_string()));
        let operation = map_command_error(SdkError::Operation("task unavailable".to_string()));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!("validation error: invalid fixture")
        );
        assert_eq!(
            serde_json::to_value(repository).expect("repository"),
            serde_json::json!("storage error: lock unavailable")
        );
        assert_eq!(
            serde_json::to_value(operation).expect("operation"),
            serde_json::json!("storage error: task unavailable")
        );
    }

    #[test]
    fn plugin_integration_errors_keep_command_safe_string_contracts() {
        use crate::contexts::tooling::plugin_integrations::domain::PluginIntegrationDomainError;

        let validation = map_command_error(PluginIntegrationError::Domain(
            PluginIntegrationDomainError::UnknownIntegration("gitlab".to_string()),
        ));
        let logging = map_command_error(PluginIntegrationError::Logging(
            "diagnostic unavailable".to_string(),
        ));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!("validation error: Unknown plugin integration: gitlab")
        );
        assert_eq!(
            serde_json::to_value(logging).expect("logging"),
            serde_json::json!("storage error: diagnostic unavailable")
        );
    }

    #[test]
    fn extension_application_errors_preserve_legacy_tauri_strings() {
        let conflict = map_command_error(ExtensionError::ConcurrentMutation(
            "an extension operation is already running for paddleocr".to_string(),
        ));
        let repository = map_command_error(ExtensionError::Repository(
            "fixture database failure".to_string(),
        ));
        let runtime = map_command_error(ExtensionError::Runtime("sidecar unavailable".to_string()));

        assert_eq!(
            serde_json::to_value(conflict).expect("conflict"),
            serde_json::json!(
                "validation error: an extension operation is already running for paddleocr"
            )
        );
        assert_eq!(
            serde_json::to_value(repository).expect("repository"),
            serde_json::json!("database error: fixture database failure")
        );
        assert_eq!(
            serde_json::to_value(runtime).expect("runtime"),
            serde_json::json!("launch failed: sidecar unavailable")
        );
    }

    #[test]
    fn skill_application_errors_preserve_legacy_tauri_strings() {
        let validation = map_command_error(SkillError::Domain(SkillDomainError::InvalidId));
        let missing = map_command_error(SkillError::NotFound("missing-skill".to_string()));
        let conflict = map_command_error(SkillError::Conflict("existing-skill".to_string()));
        let unknown_agent = map_command_error(SkillError::Domain(SkillDomainError::UnknownAgent(
            "unknown-agent".to_string(),
        )));
        let repository = map_command_error(SkillError::Repository("fixture failure".to_string()));
        let filesystem = map_command_error(SkillError::Filesystem("file failure".to_string()));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!(
                "validation error: Skill id must be kebab-case letters, digits, and hyphens"
            )
        );
        assert_eq!(
            serde_json::to_value(missing).expect("missing"),
            serde_json::json!("validation error: Skill not found: missing-skill")
        );
        assert_eq!(
            serde_json::to_value(conflict).expect("conflict"),
            serde_json::json!("validation error: Skill already exists: existing-skill")
        );
        assert_eq!(
            serde_json::to_value(unknown_agent).expect("unknown agent"),
            serde_json::json!("agent not found: unknown-agent")
        );
        assert_eq!(
            serde_json::to_value(repository).expect("repository"),
            serde_json::json!("database error: fixture failure")
        );
        assert_eq!(
            serde_json::to_value(filesystem).expect("filesystem"),
            serde_json::json!("storage error: file failure")
        );
    }

    #[test]
    fn prompt_hook_application_errors_preserve_legacy_tauri_strings() {
        let validation = map_command_error(PromptHookError::Domain(
            PromptHookDomainError::CannotBeDisabled,
        ));
        let missing = map_command_error(PromptHookError::NotFound("missing-hook".to_string()));
        let conflict = map_command_error(PromptHookError::Conflict("existing-hook".to_string()));
        let repository =
            map_command_error(PromptHookError::Repository("fixture failure".to_string()));

        assert_eq!(
            serde_json::to_value(validation).expect("validation"),
            serde_json::json!("validation error: Prompt Hook cannot be disabled.")
        );
        assert_eq!(
            serde_json::to_value(missing).expect("missing"),
            serde_json::json!("validation error: Prompt Hook not found: missing-hook")
        );
        assert_eq!(
            serde_json::to_value(conflict).expect("conflict"),
            serde_json::json!("validation error: Prompt Hook already exists: existing-hook")
        );
        assert_eq!(
            serde_json::to_value(repository).expect("repository"),
            serde_json::json!("database error: fixture failure")
        );
    }

    #[test]
    fn desktop_application_errors_preserve_legacy_tauri_strings() {
        use crate::contexts::desktop::domain::DesktopSettingMutation;

        let domain_error =
            DesktopSettingMutation::parse("fontSize", "20px").expect_err("domain error");
        let validation = map_command_error(DesktopSettingsError::Domain(domain_error));
        let database = map_command_error(DesktopSettingsError::Repository(
            "fixture failure".to_string(),
        ));
        let proxy = map_command_error(DesktopSettingsError::NetworkProxy(
            "launch failed: connection refused".to_string(),
        ));
        let startup = map_command_error(DesktopSettingsError::Startup(
            "permission denied".to_string(),
        ));

        assert_eq!(
            validation.message(),
            "validation error: Invalid setting value for key 'fontSize'."
        );
        assert_eq!(database.message(), "database error: fixture failure");
        assert_eq!(proxy.message(), "launch failed: connection refused");
        assert_eq!(startup.message(), "storage error: permission denied");
        assert_eq!(proxy.category(), CommandErrorCategory::Unavailable);
    }

    #[test]
    fn floating_assistant_errors_preserve_legacy_tauri_strings() {
        use crate::contexts::desktop::domain::FloatingAssistantDomainError;

        let validation = map_command_error(FloatingAssistantError::Domain(
            FloatingAssistantDomainError::UnsupportedPlatform,
        ));
        let database = map_command_error(FloatingAssistantError::Repository(
            "database error: fixture failure".to_string(),
        ));
        let window = map_command_error(FloatingAssistantError::Window(
            "window unavailable".to_string(),
        ));

        assert_eq!(
            validation.message(),
            "validation error: floating assistant is currently available on Windows only"
        );
        assert_eq!(database.message(), "database error: fixture failure");
        assert_eq!(window.message(), "storage error: window unavailable");
    }

    #[test]
    fn workspace_errors_preserve_legacy_validation_and_storage_prefixes() {
        use crate::contexts::workspaces::domain::WorkspaceDomainError;

        let domain = map_command_error(WorkspaceError::Domain(
            WorkspaceDomainError::InvalidWorktreeName,
        ));
        let repository = map_command_error(WorkspaceError::Repository(
            "fixture database failure".to_string(),
        ));
        let selection =
            map_command_error(WorkspaceError::Selection("dialog unavailable".to_string()));
        let storage = map_command_error(WorkspaceError::Storage("file unavailable".to_string()));
        let launch = map_command_error(WorkspaceError::LaunchFailed("Git failed".to_string()));
        let missing = map_command_error(WorkspaceError::SessionNotFound("session-1".to_string()));

        assert_eq!(domain.message(), "validation error: Invalid worktree name");
        assert_eq!(
            repository.message(),
            "database error: fixture database failure"
        );
        assert_eq!(selection.message(), "storage error: dialog unavailable");
        assert_eq!(storage.message(), "storage error: file unavailable");
        assert_eq!(launch.message(), "launch failed: Git failed");
        assert_eq!(launch.category(), CommandErrorCategory::Unavailable);
        assert_eq!(missing.message(), "session not found: session-1");
        assert_eq!(missing.category(), CommandErrorCategory::NotFound);
    }

    #[test]
    fn session_errors_preserve_legacy_tauri_strings() {
        let missing = map_command_error(SessionsError::SessionNotFound("session-1".to_string()));
        let missing_message =
            map_command_error(SessionsError::MessageNotFound("message-1".to_string()));
        let category = map_command_error(SessionsError::CategoryNotFound("category-1".to_string()));
        let conflict = map_command_error(SessionsError::CategoryNameConflict("Work".to_string()));
        let database = map_command_error(SessionsError::Repository("fixture failure".to_string()));
        let runtime = map_command_error(SessionsError::Runtime("cleanup failed".to_string()));

        assert_eq!(missing.message(), "session not found: session-1");
        assert_eq!(missing.category(), CommandErrorCategory::NotFound);
        assert_eq!(
            missing_message.message(),
            "validation error: Message not found: message-1"
        );
        assert_eq!(missing_message.category(), CommandErrorCategory::Validation);
        assert_eq!(
            category.message(),
            "validation error: Category not found: category-1"
        );
        assert_eq!(
            conflict.message(),
            "validation error: Category name already exists."
        );
        assert_eq!(database.message(), "database error: fixture failure");
        assert_eq!(runtime.message(), "storage error: cleanup failed");
    }

    #[test]
    fn agent_runtime_errors_preserve_legacy_tauri_strings() {
        let missing = map_command_error(AgentRuntimeApplicationError::AgentNotFound(
            "missing-agent".to_string(),
        ));
        let unavailable = map_command_error(AgentRuntimeApplicationError::AgentUnavailable(
            "Command 'codex' was not found on PATH.".to_string(),
        ));
        let conflict = map_command_error(AgentRuntimeApplicationError::GenerationConflict(
            "session-1".to_string(),
        ));
        let launch = map_command_error(AgentRuntimeApplicationError::Process(
            "Command 'codex' was not found on PATH.".to_string(),
        ));

        assert_eq!(missing.message(), "agent not found: missing-agent");
        assert_eq!(missing.category(), CommandErrorCategory::NotFound);
        assert_eq!(
            unavailable.message(),
            "agent is unavailable: Command 'codex' was not found on PATH."
        );
        assert_eq!(
            conflict.message(),
            "validation error: A generation is already active for session session-1."
        );
        assert_eq!(
            launch.message(),
            "launch failed: Command 'codex' was not found on PATH."
        );
    }

    #[test]
    fn communications_errors_preserve_safe_im_command_strings() {
        let failure = map_command_error(CommunicationsApplicationError::failure(
            "connector-credentials-required",
        ));
        let domain = map_command_error(CommunicationsApplicationError::Domain(
            crate::contexts::communications::domain::CommunicationsDomainError::RequiredValue(
                "Routing agent id",
            ),
        ));

        assert_eq!(failure.message(), "connector-credentials-required");
        assert_eq!(
            serde_json::to_value(failure).expect("serialize failure"),
            serde_json::json!("connector-credentials-required")
        );
        assert_eq!(domain.message(), "communications-domain-invalid");
        assert_eq!(domain.category(), CommandErrorCategory::Validation);
    }
}
