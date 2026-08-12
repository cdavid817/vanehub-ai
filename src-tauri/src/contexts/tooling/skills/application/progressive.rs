use super::{SkillApplicationError, SkillPackageResource, SkillResourceEntry, SkillResourceIndex};
use crate::contexts::tooling::skills::domain::SkillId;
use std::path::{Component, Path};

pub(crate) const MAX_DISCOVERY_RESULTS: usize = 100;
pub(crate) const MAX_DISCOVERY_QUERY_CHARACTERS: usize = 80;
pub(crate) const MAX_INLINE_SKILL_CHARACTERS: usize = 12_000;
pub(crate) const MAX_RESOURCE_ENTRIES: usize = 128;
pub(crate) const MAX_RESOURCE_PATH_CHARACTERS: usize = 240;
pub(crate) const MAX_LOGICAL_URI_CHARACTERS: usize = 512;
pub(crate) const MAX_RESOURCE_BYTES: u64 = 65_536;
pub(crate) const MAX_RESOURCE_CHARACTERS: usize = 32_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSkillUri {
    pub(crate) id: SkillId,
    pub(crate) relative_path: Option<String>,
}

pub(crate) fn logical_base_uri(id: &SkillId) -> String {
    format!("skill://{}/", id.as_str())
}

pub(crate) fn logical_resource_uri(
    id: &SkillId,
    relative_path: &str,
) -> Result<String, SkillApplicationError> {
    validate_relative_resource_path(relative_path)?;
    let uri = format!("skill://{}/{}", id.as_str(), relative_path);
    if uri.chars().count() > MAX_LOGICAL_URI_CHARACTERS {
        return Err(SkillApplicationError::InvalidResourceUri);
    }
    Ok(uri)
}

pub(crate) fn parse_logical_uri(uri: &str) -> Result<ParsedSkillUri, SkillApplicationError> {
    if uri.chars().count() > MAX_LOGICAL_URI_CHARACTERS || uri.contains(['?', '#', '\\', '%']) {
        return Err(invalid_uri());
    }
    let remainder = uri.strip_prefix("skill://").ok_or_else(invalid_uri)?;
    let (id, path) = remainder.split_once('/').ok_or_else(invalid_uri)?;
    let id = SkillId::parse(id).map_err(|_| invalid_uri())?;
    if path.is_empty() {
        return Ok(ParsedSkillUri {
            id,
            relative_path: None,
        });
    }
    validate_relative_resource_path(path)?;
    Ok(ParsedSkillUri {
        id,
        relative_path: Some(path.to_string()),
    })
}

pub(crate) fn build_resource_index(
    id: &SkillId,
    mut resources: Vec<SkillPackageResource>,
) -> Result<SkillResourceIndex, SkillApplicationError> {
    resources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    resources.dedup_by(|left, right| left.relative_path == right.relative_path);
    let truncated = resources.len() > MAX_RESOURCE_ENTRIES;
    resources.truncate(MAX_RESOURCE_ENTRIES);
    let mut index = SkillResourceIndex {
        truncated,
        ..SkillResourceIndex::default()
    };
    for resource in resources {
        validate_relative_resource_path(&resource.relative_path)?;
        let entry = SkillResourceEntry {
            uri: logical_resource_uri(id, &resource.relative_path)?,
            relative_path: resource.relative_path.clone(),
            size_bytes: resource.size_bytes,
        };
        match resource.relative_path.split('/').next() {
            Some("scripts") => index.scripts.push(entry),
            Some("references") => index.references.push(entry),
            Some("templates") => index.templates.push(entry),
            Some("assets") => index.assets.push(entry),
            _ => return Err(invalid_uri()),
        }
    }
    Ok(index)
}

pub(crate) fn truncate_chars(value: &str, limit: usize) -> (String, bool) {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(limit).collect::<String>();
    (prefix, chars.next().is_some())
}

pub(crate) fn validate_relative_resource_path(value: &str) -> Result<(), SkillApplicationError> {
    if value.is_empty()
        || value.chars().count() > MAX_RESOURCE_PATH_CHARACTERS
        || value.starts_with(['/', '\\'])
        || value.contains('\\')
        || value.contains('%')
        || value.contains(['\0', '\r', '\n'])
        || value.split('/').any(str::is_empty)
    {
        return Err(invalid_uri());
    }
    let path = Path::new(value);
    let mut components = path.components();
    let first = components.next();
    if !matches!(
        first,
        Some(Component::Normal(value))
            if matches!(value.to_str(), Some("scripts" | "references" | "templates" | "assets"))
    ) {
        return Err(invalid_uri());
    }
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(invalid_uri());
        };
        let component = component.to_str().ok_or_else(invalid_uri)?;
        if component.is_empty() || component.starts_with('.') {
            return Err(invalid_uri());
        }
    }
    Ok(())
}

fn invalid_uri() -> SkillApplicationError {
    SkillApplicationError::InvalidResourceUri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_uris_are_host_path_free_and_reject_unsafe_components() {
        let id = SkillId::parse("safe-skill").expect("id");
        assert_eq!(logical_base_uri(&id), "skill://safe-skill/");
        assert_eq!(
            logical_resource_uri(&id, "references/guide.md").expect("uri"),
            "skill://safe-skill/references/guide.md"
        );
        for invalid in [
            "C:/secret.txt",
            "/etc/passwd",
            "references/../secret.md",
            "references/.hidden.md",
            "references\\guide.md",
            "references/%2e%2e/secret.md",
        ] {
            assert!(
                validate_relative_resource_path(invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn unicode_truncation_never_splits_a_character() {
        let (value, truncated) = truncate_chars("甲乙丙", 2);
        assert_eq!(value, "甲乙");
        assert!(truncated);
    }

    #[test]
    fn resource_indexes_are_sorted_and_bounded() {
        let id = SkillId::parse("bounded-skill").expect("id");
        let resources = (0..140)
            .rev()
            .map(|index| SkillPackageResource {
                relative_path: format!("references/{index:03}.md"),
                media_type: "text/markdown".to_string(),
                size_bytes: 1,
                content_hash: format!("hash-{index:03}"),
            })
            .collect();
        let index = build_resource_index(&id, resources).expect("index");
        assert_eq!(index.references.len(), MAX_RESOURCE_ENTRIES);
        assert_eq!(index.references[0].relative_path, "references/000.md");
        assert!(index.truncated);
    }
}
