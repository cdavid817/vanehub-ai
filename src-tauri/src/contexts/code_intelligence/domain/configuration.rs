use super::models::{DomainModelError, LanguageFamily};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const MAX_INITIALIZATION_OPTIONS_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageConfiguration {
    pub(crate) enabled: bool,
    pub(crate) executable_override: Option<String>,
    pub(crate) initialization_options: Value,
}

impl Default for LanguageConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            executable_override: None,
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
    pub(crate) languages: BTreeMap<LanguageFamily, LanguageConfiguration>,
}

impl Default for LspConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            languages: [
                (LanguageFamily::Rust, LanguageConfiguration::default()),
                (
                    LanguageFamily::TypeScriptJavaScript,
                    LanguageConfiguration::default(),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }
}

impl LspConfiguration {
    pub(crate) fn validate(&self) -> Result<(), DomainModelError> {
        if self.languages.len() != 2
            || !self.languages.contains_key(&LanguageFamily::Rust)
            || !self
                .languages
                .contains_key(&LanguageFamily::TypeScriptJavaScript)
        {
            return Err(DomainModelError::IncompleteLanguageConfiguration);
        }
        self.languages
            .values()
            .try_for_each(LanguageConfiguration::validate)
    }
}
