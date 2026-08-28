use super::language_id::LspLanguageId;
use super::models::DomainModelError;
use super::registry::LANGUAGE_DEFINITIONS;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const MAX_INITIALIZATION_OPTIONS_BYTES: usize = 32 * 1024;
/// Enough for any real server invocation while keeping a pasted blob from becoming a process
/// argument list.
pub(crate) const MAX_STARTUP_ARGUMENTS: usize = 32;
pub(crate) const MAX_STARTUP_ARGUMENT_BYTES: usize = 4 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageConfiguration {
    pub(crate) enabled: bool,
    pub(crate) executable_override: Option<String>,
    /// `None` means "use the arguments the registry declares for this language". An empty vector
    /// means the user chose to pass none. Collapsing the two would strip `--stdio` from the
    /// TypeScript server the moment someone cleared the field.
    pub(crate) startup_arguments: Option<Vec<String>>,
    pub(crate) initialization_options: Value,
}

impl Default for LanguageConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            executable_override: None,
            startup_arguments: None,
            initialization_options: Value::Object(Map::new()),
        }
    }
}

impl LanguageConfiguration {
    fn validate(&self) -> Result<(), DomainModelError> {
        if self
            .executable_override
            .as_deref()
            .is_some_and(|path| path.trim().is_empty() || !Path::new(path).is_absolute())
        {
            return Err(DomainModelError::InvalidExecutableOverride);
        }
        if let Some(arguments) = &self.startup_arguments {
            if arguments.len() > MAX_STARTUP_ARGUMENTS
                || arguments.iter().map(String::len).sum::<usize>() > MAX_STARTUP_ARGUMENT_BYTES
                || arguments.iter().any(|argument| argument.contains('\0'))
            {
                return Err(DomainModelError::InvalidStartupArguments);
            }
        }
        if !self.initialization_options.is_object()
            || serde_json::to_vec(&self.initialization_options)
                .map_err(|_| DomainModelError::InvalidInitializationOptions)?
                .len()
                > MAX_INITIALIZATION_OPTIONS_BYTES
        {
            return Err(DomainModelError::InvalidInitializationOptions);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LspConfiguration {
    pub(crate) enabled: bool,
    pub(crate) languages: BTreeMap<LspLanguageId, LanguageConfiguration>,
}

impl Default for LspConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            languages: LANGUAGE_DEFINITIONS
                .iter()
                .map(|definition| (definition.language_id(), LanguageConfiguration::default()))
                .collect(),
        }
    }
}

impl LspConfiguration {
    /// Validates each language's settings. It deliberately does not require the configuration to
    /// name every registered language: the set is no longer fixed, so a build that registers a new
    /// language must be able to read a configuration written before it existed.
    pub(crate) fn validate(&self) -> Result<(), DomainModelError> {
        self.languages
            .values()
            .try_for_each(LanguageConfiguration::validate)
    }

    pub(crate) fn language(&self, language_id: &LspLanguageId) -> Option<&LanguageConfiguration> {
        self.languages.get(language_id)
    }
}
