use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandTemplateScope {
    Global,
    Connection,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandTemplate {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) scope: CommandTemplateScope,
    pub(crate) connection_id: Option<String>,
    pub(crate) workspace_uri: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) tags_json: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum CommandTemplateError {
    #[error("command template has invalid scope binding")]
    InvalidScope,
    #[error("command template contains a secret-like pattern")]
    SecretLikeCommand,
    #[error("command template field is empty or too long")]
    InvalidField,
}

impl CommandTemplate {
    pub(crate) fn validate(&self) -> Result<(), CommandTemplateError> {
        if self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.command.trim().is_empty()
            || self.name.len() > 200
            || self.command.len() > 16_384
        {
            return Err(CommandTemplateError::InvalidField);
        }
        let valid_scope = match self.scope {
            CommandTemplateScope::Global => {
                self.connection_id.is_none() && self.workspace_uri.is_none()
            }
            CommandTemplateScope::Connection => {
                self.connection_id.is_some() && self.workspace_uri.is_none()
            }
            CommandTemplateScope::Workspace => {
                self.connection_id.is_none() && self.workspace_uri.is_some()
            }
        };
        if !valid_scope {
            return Err(CommandTemplateError::InvalidScope);
        }
        let lower = self.command.to_ascii_lowercase();
        if ["password=", "token=", "api_key=", "private_key", "secret="]
            .iter()
            .any(|pattern| lower.contains(pattern))
        {
            return Err(CommandTemplateError::SecretLikeCommand);
        }
        Ok(())
    }
}
