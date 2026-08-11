#![cfg_attr(not(test), allow(dead_code))]

use super::DEFAULT_OVERLAY_LIMITS;

const ALLOWED_TOP_LEVELS: &[&str] = &["references", "templates", "assets"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayPathError {
    Empty,
    AbsolutePath,
    ParentTraversal,
    CurrentDirectory,
    HiddenComponent,
    ReservedDevice,
    AlternateDataStream,
    UnsupportedTopLevel,
    NonCanonicalSeparator,
    EmptyComponent,
    TrailingDotOrSpace,
    InvalidCharacter,
    MissingFileName,
    TooLong { maximum: usize, actual: usize },
    TooDeep { maximum: usize, actual: usize },
    LinkEscape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedOverlayPath(String);

impl ValidatedOverlayPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn validate_overlay_path(value: &str) -> Result<ValidatedOverlayPath, OverlayPathError> {
    if value.is_empty() {
        return Err(OverlayPathError::Empty);
    }
    if is_absolute_like(value) {
        return Err(OverlayPathError::AbsolutePath);
    }
    let security_components = value.replace('\\', "/");
    if security_components
        .split('/')
        .any(|component| component == "..")
    {
        return Err(OverlayPathError::ParentTraversal);
    }
    if value.contains('\\') {
        return Err(OverlayPathError::NonCanonicalSeparator);
    }
    let character_count = value.chars().count();
    if character_count > DEFAULT_OVERLAY_LIMITS.maximum_path_characters {
        return Err(OverlayPathError::TooLong {
            maximum: DEFAULT_OVERLAY_LIMITS.maximum_path_characters,
            actual: character_count,
        });
    }

    let components = value.split('/').collect::<Vec<_>>();
    if components.len() < 2 {
        return Err(OverlayPathError::MissingFileName);
    }
    if components.len() > DEFAULT_OVERLAY_LIMITS.maximum_path_depth {
        return Err(OverlayPathError::TooDeep {
            maximum: DEFAULT_OVERLAY_LIMITS.maximum_path_depth,
            actual: components.len(),
        });
    }
    if !ALLOWED_TOP_LEVELS.contains(&components[0]) {
        if components[0].starts_with('.') {
            return Err(OverlayPathError::HiddenComponent);
        }
        return Err(OverlayPathError::UnsupportedTopLevel);
    }
    for component in components {
        validate_component(component)?;
    }
    Ok(ValidatedOverlayPath(value.to_string()))
}

pub(crate) fn validate_overlay_link_target(
    link_path: &str,
    target: &str,
) -> Result<ValidatedOverlayPath, OverlayPathError> {
    let link_path = validate_overlay_path(link_path)?;
    if target.is_empty() {
        return Err(OverlayPathError::Empty);
    }
    if is_absolute_like(target) {
        return Err(OverlayPathError::AbsolutePath);
    }
    if target.contains('\\') {
        return Err(OverlayPathError::NonCanonicalSeparator);
    }

    let mut resolved = link_path
        .as_str()
        .split('/')
        .map(str::to_string)
        .collect::<Vec<_>>();
    resolved.pop();
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if resolved.pop().is_none() {
                    return Err(OverlayPathError::LinkEscape);
                }
            }
            component => {
                validate_component(component)?;
                resolved.push(component.to_string());
            }
        }
    }
    if resolved.is_empty() {
        return Err(OverlayPathError::LinkEscape);
    }
    validate_overlay_path(&resolved.join("/"))
        .map_err(|error| map_resolved_link_error(error, resolved.first().map(String::as_str)))
}

fn validate_component(component: &str) -> Result<(), OverlayPathError> {
    if component.is_empty() {
        return Err(OverlayPathError::EmptyComponent);
    }
    if component == "." {
        return Err(OverlayPathError::CurrentDirectory);
    }
    if component == ".." {
        return Err(OverlayPathError::ParentTraversal);
    }
    if component.starts_with('.') {
        return Err(OverlayPathError::HiddenComponent);
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return Err(OverlayPathError::TrailingDotOrSpace);
    }
    if component.contains(':') {
        return Err(OverlayPathError::AlternateDataStream);
    }
    if component.chars().any(|character| {
        character.is_control() || matches!(character, '<' | '>' | '"' | '|' | '?' | '*')
    }) {
        return Err(OverlayPathError::InvalidCharacter);
    }
    if is_windows_reserved_device(component) {
        return Err(OverlayPathError::ReservedDevice);
    }
    Ok(())
}

fn is_absolute_like(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with('\\')
        || (value.len() >= 2
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':')
}

fn is_windows_reserved_device(component: &str) -> bool {
    let stem = component
        .split_once('.')
        .map_or(component, |(before_extension, _)| before_extension)
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || is_numbered_device(&stem, "COM")
        || is_numbered_device(&stem, "LPT")
}

fn is_numbered_device(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .is_some_and(|suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
}

fn map_resolved_link_error(error: OverlayPathError, top_level: Option<&str>) -> OverlayPathError {
    if matches!(
        error,
        OverlayPathError::UnsupportedTopLevel | OverlayPathError::MissingFileName
    ) || top_level.is_none()
    {
        OverlayPathError::LinkEscape
    } else {
        error
    }
}
