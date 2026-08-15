use crate::contexts::tooling::skills::application::{SkillApplicationError, SkillDocument};
use crate::contexts::tooling::skills::domain::{
    RawSkillDelegation, SkillDelegationDeclaration, SkillMetadata,
};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

const MAX_IMPORT_FILES: usize = 512;
const MAX_IMPORT_DEPTH: usize = 16;
const MAX_IMPORT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SKILL_DOCUMENT_BYTES: u64 = 256 * 1024;

pub(crate) fn compose(document: &SkillDocument) -> String {
    let triggers = document
        .metadata
        .triggers
        .iter()
        .map(|trigger| format!("  - {trigger}"))
        .collect::<Vec<_>>()
        .join("\n");
    let aliases = document
        .metadata
        .aliases
        .iter()
        .map(|alias| format!("  - {}", alias.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nid: {}\nname: {}\ndescription: {}\ncategory: {}\nversion: {}\ntype: {}\ndelivery: {}\ntriggers:\n{}\naliases:\n{}{}\n---\n\n# {}\n\n{}\n",
        document.metadata.id.as_str(),
        document.metadata.name,
        document.metadata.description,
        document.metadata.category,
        document.metadata.version,
        document.metadata.skill_type.as_str(),
        document.metadata.delivery.as_str(),
        triggers,
        aliases,
        compose_delegation(&document.metadata.delegation),
        document.metadata.name,
        document.body.trim()
    )
}

/// Round-trips the declared block verbatim. Composition must not normalize an invalid contract
/// into a valid-looking one, because the package author has to see what they wrote to repair it.
fn compose_delegation(declaration: &SkillDelegationDeclaration) -> String {
    let Some(raw) = declaration.raw() else {
        return String::new();
    };
    let mut block = String::from("\ndelegation:");
    if !raw.tools.is_empty() {
        block.push_str("\n  tools:");
        for tool in &raw.tools {
            block.push_str(&format!("\n    - {tool}"));
        }
    }
    for (key, value) in &raw.fields {
        block.push_str(&format!("\n  {key}: {value}"));
    }
    block
}

pub(super) fn parse(content: &str) -> Result<SkillMetadata, SkillApplicationError> {
    let normalized = content.replace("\r\n", "\n");
    let frontmatter = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(frontmatter, _)| frontmatter)
        .ok_or_else(|| validation_error("SKILL.md requires frontmatter"))?;
    let mut id = String::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut category = String::new();
    let mut version = String::new();
    let mut triggers = Vec::new();
    let mut aliases = Vec::new();
    let mut skill_type = None;
    let mut delivery = None;
    let mut delegation: Option<RawSkillDelegation> = None;
    let mut list_key: Option<&str> = None;
    let mut in_delegation = false;
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(key) = list_key.filter(|_| line.starts_with('-')) {
            let value = line
                .trim_start_matches('-')
                .trim()
                .trim_matches('"')
                .to_string();
            match key {
                "triggers" => triggers.push(value),
                "aliases" => aliases.push(value),
                "tools" => {
                    if let Some(block) = delegation.as_mut() {
                        block.tools.push(value);
                    }
                }
                _ => {}
            }
            continue;
        }
        list_key = None;
        // Indentation is the only signal separating the nested `delegation` contract from the
        // flat frontmatter keys, so it has to be read before the line is trimmed.
        let nested = raw_line.starts_with(' ') || raw_line.starts_with('\t');
        if in_delegation && nested {
            if line == "tools:" {
                list_key = Some("tools");
                continue;
            }
            if let (Some((key, value)), Some(block)) = (line.split_once(':'), delegation.as_mut()) {
                block.fields.insert(
                    key.trim().to_string(),
                    value.trim().trim_matches('"').to_string(),
                );
            }
            continue;
        }
        in_delegation = false;
        if line == "delegation:" {
            in_delegation = true;
            delegation = Some(RawSkillDelegation::default());
            continue;
        }
        if line == "triggers:" || line == "aliases:" {
            list_key = Some(line.trim_end_matches(':'));
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "id" => id = value,
            "name" => name = value,
            "description" => description = value,
            "category" => category = value,
            "version" => version = value,
            "type" => {
                skill_type = Some(
                    crate::contexts::tooling::skills::domain::SkillType::parse(&value)
                        .map_err(|error| validation_error(error.to_string()))?,
                )
            }
            "delivery" => {
                delivery = Some(
                    crate::contexts::tooling::skills::domain::SkillDelivery::parse(&value)
                        .map_err(|error| validation_error(error.to_string()))?,
                )
            }
            _ => {}
        }
    }
    let metadata = SkillMetadata::with_classification(
        id,
        name,
        description,
        category,
        version,
        triggers,
        aliases,
        skill_type,
        delivery,
    )
    .map_err(|error| validation_error(error.to_string()))?;
    Ok(match delegation {
        Some(block) => metadata.with_delegation(SkillDelegationDeclaration::declared(block)),
        None => metadata,
    })
}

pub(crate) fn parse_document(content: &str) -> Result<SkillDocument, SkillApplicationError> {
    let metadata = parse(content)?;
    let normalized = content.replace("\r\n", "\n");
    let raw_body = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(_, remainder)| remainder.trim())
        .ok_or_else(|| validation_error("SKILL.md requires frontmatter"))?;
    let heading = format!("# {}\n\n", metadata.name);
    let body = raw_body
        .strip_prefix(&heading)
        .unwrap_or(raw_body)
        .trim()
        .to_string();
    Ok(SkillDocument { metadata, body })
}

pub(crate) fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(super) fn copy_directory(source: &Path, target: &Path) -> Result<(), SkillApplicationError> {
    let mut budget = ImportBudget::default();
    copy_directory_bounded(source, target, 0, &mut budget)
}

pub(crate) fn read_import_document(path: &Path) -> Result<String, SkillApplicationError> {
    let size = std::fs::metadata(path).map_err(filesystem_error)?.len();
    if size > MAX_SKILL_DOCUMENT_BYTES {
        return Err(validation_error("SKILL.md exceeds 256 KiB"));
    }
    std::fs::read_to_string(path).map_err(filesystem_error)
}

#[derive(Default)]
struct ImportBudget {
    files: usize,
    bytes: u64,
}

fn copy_directory_bounded(
    source: &Path,
    target: &Path,
    depth: usize,
    budget: &mut ImportBudget,
) -> Result<(), SkillApplicationError> {
    if depth > MAX_IMPORT_DEPTH {
        return Err(validation_error(
            "Invalid Skill source: import depth exceeds 16",
        ));
    }
    std::fs::create_dir_all(target).map_err(filesystem_error)?;
    for entry in std::fs::read_dir(source).map_err(filesystem_error)? {
        let entry = entry.map_err(filesystem_error)?;
        let file_type = entry.file_type().map_err(filesystem_error)?;
        let destination = target.join(entry.file_name());
        if file_type.is_symlink() {
            return Err(validation_error(
                "Invalid Skill source: symbolic links are not supported",
            ));
        }
        if file_type.is_dir() {
            copy_directory_bounded(&entry.path(), &destination, depth + 1, budget)?;
        } else if file_type.is_file() {
            budget.files += 1;
            let size = entry.metadata().map_err(filesystem_error)?.len();
            budget.bytes = budget.bytes.saturating_add(size);
            if budget.files > MAX_IMPORT_FILES {
                return Err(validation_error(
                    "Invalid Skill source: import file count exceeds 512",
                ));
            }
            if budget.bytes > MAX_IMPORT_BYTES {
                return Err(validation_error(
                    "Invalid Skill source: import size exceeds 16 MiB",
                ));
            }
            if entry.file_name() == "SKILL.md" && size > MAX_SKILL_DOCUMENT_BYTES {
                return Err(validation_error(
                    "Invalid Skill source: SKILL.md exceeds 256 KiB",
                ));
            }
            std::fs::copy(entry.path(), destination).map_err(filesystem_error)?;
        }
    }
    Ok(())
}

fn validation_error(message: impl Into<String>) -> SkillApplicationError {
    SkillApplicationError::Validation(message.into())
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{SkillDelivery, SkillType};

    #[test]
    fn legacy_document_parsing_records_compatibility_defaults() {
        let metadata = parse(
            "---\nid: legacy-skill\nname: Legacy\ndescription: Existing\ncategory: test\nversion: 1.0.0\ntriggers:\n  - legacy\n---\n\nBody",
        )
        .expect("legacy metadata");

        assert_eq!(metadata.skill_type, SkillType::Role);
        assert_eq!(metadata.delivery, SkillDelivery::Eager);
        assert!(metadata.compatibility_defaults.skill_type);
        assert!(metadata.compatibility_defaults.delivery);
    }

    #[test]
    fn explicit_classification_and_aliases_round_trip() {
        let document = SkillDocument {
            metadata: SkillMetadata::with_classification(
                "developer",
                "Developer",
                "Development role",
                "development",
                "1.0.0",
                vec!["develop".to_string()],
                vec!["dev".to_string()],
                Some(SkillType::Role),
                Some(SkillDelivery::OnDemand),
            )
            .expect("metadata"),
            body: "Use {skill_base_dir}.".to_string(),
        };

        let parsed = parse(&compose(&document)).expect("round trip");

        assert_eq!(parsed.aliases[0].as_str(), "dev");
        assert_eq!(parsed.skill_type, SkillType::Role);
        assert_eq!(parsed.delivery, SkillDelivery::OnDemand);
        assert_eq!(parsed.compatibility_defaults, Default::default());
    }

    #[test]
    fn delegation_contract_is_parsed_and_round_trips_through_composition() {
        let source = "---\nid: code-explorer\nname: Code Explorer\ndescription: Explores code\ncategory: development\nversion: 1.0.0\ntype: utility\ndelivery: on-demand\ntriggers:\n  - explore\naliases:\n  - explorer\ndelegation:\n  tools:\n    - file-read\n    - content-search\n  max_rounds: 6\n  timeout_seconds: 90\n---\n\nBody";

        let metadata = parse(source).expect("metadata");
        let raw = metadata.delegation.raw().expect("declared delegation");
        assert_eq!(raw.tools, vec!["file-read", "content-search"]);
        assert_eq!(raw.fields.get("max_rounds").map(String::as_str), Some("6"));
        assert_eq!(
            raw.fields.get("timeout_seconds").map(String::as_str),
            Some("90")
        );
        assert_eq!(metadata.triggers, vec!["explore"]);
        assert_eq!(metadata.aliases[0].as_str(), "explorer");

        let composed = compose(&SkillDocument {
            metadata: metadata.clone(),
            body: "Body".to_string(),
        });
        assert_eq!(parse(&composed).expect("round trip"), metadata);
    }

    #[test]
    fn invalid_delegation_block_keeps_the_skill_parseable() {
        let metadata = parse(
            "---\nid: broken-utility\nname: Broken\ndescription: Broken\ncategory: test\nversion: 1.0.0\ntype: utility\ndelegation:\n  tools:\n    - launch-rockets\n---\n\nBody",
        )
        .expect("metadata");

        assert_eq!(metadata.id.as_str(), "broken-utility");
        assert_eq!(
            metadata.delegation.raw().expect("declared").tools,
            vec!["launch-rockets"]
        );
    }

    #[test]
    fn absent_delegation_block_stays_absent() {
        let metadata = parse(
            "---\nid: plain-utility\nname: Plain\ndescription: Plain\ncategory: test\nversion: 1.0.0\ntype: utility\ntriggers:\n  - plain\n---\n\nBody",
        )
        .expect("metadata");

        assert!(metadata.delegation.is_absent());
        assert_eq!(metadata.triggers, vec!["plain"]);
    }

    #[test]
    fn unknown_classification_is_rejected_without_legacy_fallback() {
        let result = parse(
            "---\nid: invalid-skill\nname: Invalid\ndescription: Invalid\ncategory: test\nversion: 1.0.0\ntype: agent\n---\n\nBody",
        );

        assert!(matches!(
            result,
            Err(SkillApplicationError::Validation(message)) if message.contains("Unknown Skill type")
        ));
    }
}
