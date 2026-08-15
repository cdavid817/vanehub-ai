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
    // Emitted verbatim: rewriting a Skill must not renormalize the schema, because the block is
    // part of the content hash that stored configuration is bound to.
    let config_schema = match document.metadata.config_schema_block.as_deref() {
        Some("") => "config_schema:\n".to_string(),
        Some(block) => format!("config_schema:\n{block}\n"),
        None => String::new(),
    };
    format!(
        "---\nid: {}\nname: {}\ndescription: {}\ncategory: {}\nversion: {}\ntype: {}\ndelivery: {}\ntriggers:\n{}\naliases:\n{}{}\n{}---\n\n# {}\n\n{}\n",
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
        config_schema,
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
    let raw_frontmatter = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(frontmatter, _)| frontmatter)
        .ok_or_else(|| validation_error("SKILL.md requires frontmatter"))?;
    // Lifted out before the line loop below, which trims indentation and would otherwise read the
    // schema's nested keys as top-level frontmatter.
    let (config_schema_block, frontmatter) =
        extract_indented_block(raw_frontmatter, "config_schema");
    let frontmatter = frontmatter.as_str();
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
    .map(|metadata| metadata.with_config_schema_block(config_schema_block))
    .map_err(|error| validation_error(error.to_string()))?;
    Ok(match delegation {
        Some(block) => metadata.with_delegation(SkillDelegationDeclaration::declared(block)),
        None => metadata,
    })
}

/// Splits `key:`'s indented block out of the frontmatter, returning it alongside the frontmatter
/// with those lines removed. A bare `key:` yields `Some("")`, which the schema layer rejects as an
/// unsupported declaration rather than silently reading as "no schema".
fn extract_indented_block(frontmatter: &str, key: &str) -> (Option<String>, String) {
    let header = format!("{key}:");
    let mut block: Option<String> = None;
    let mut remaining: Vec<&str> = Vec::new();
    let mut lines = frontmatter.lines().peekable();
    while let Some(line) = lines.next() {
        let is_header = block.is_none()
            && line == header
            && !line.starts_with(|character: char| character.is_whitespace());
        if !is_header {
            remaining.push(line);
            continue;
        }
        let mut collected: Vec<&str> = Vec::new();
        while let Some(next) = lines.peek() {
            let indented = next.starts_with(' ') || next.starts_with('\t');
            if !indented {
                break;
            }
            collected.push(next);
            lines.next();
        }
        block = Some(collected.join("\n"));
    }
    (block, remaining.join("\n"))
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

    const WITH_SCHEMA: &str = "---\nid: configured-skill\nname: Configured\ndescription: Has settings\ncategory: test\nversion: 1.0.0\nconfig_schema:\n  properties:\n    endpoint:\n      type: string\n      default: https://example.com\n    api_key:\n      type: string\n      x-vanehub-secret: true\ntriggers:\n  - configured\n---\n\nBody";

    #[test]
    fn config_schema_block_is_parsed_without_leaking_into_other_frontmatter_keys() {
        let metadata = parse(WITH_SCHEMA).expect("metadata");

        assert!(metadata.is_configurable());
        // The schema's nested keys must not have been read as frontmatter fields.
        assert_eq!(metadata.name, "Configured");
        assert_eq!(metadata.category, "test");
        assert_eq!(metadata.triggers, vec!["configured"]);

        let schema = metadata
            .config_schema()
            .expect("declared")
            .expect("supported schema");
        assert_eq!(schema.fields.len(), 2);
        assert!(schema.field("api_key").expect("api_key").secret);
    }

    #[test]
    fn skill_without_config_schema_stays_unconfigurable() {
        let metadata = parse(
            "---\nid: plain-skill\nname: Plain\ndescription: None\ncategory: test\nversion: 1.0.0\n---\n\nBody",
        )
        .expect("metadata");

        assert!(!metadata.is_configurable());
        assert!(metadata.config_schema().is_none());
    }

    #[test]
    fn unsupported_config_schema_loads_the_skill_but_reports_no_usable_schema() {
        let metadata = parse(
            "---\nid: broken-skill\nname: Broken\ndescription: Bad schema\ncategory: test\nversion: 1.0.0\nconfig_schema:\n  properties:\n    field:\n      type: date\n---\n\nBody",
        )
        .expect("Skill still loads");

        assert!(metadata.is_configurable());
        assert!(metadata.config_schema().expect("declared").is_err());
    }

    #[test]
    fn config_schema_survives_a_compose_parse_round_trip() {
        let metadata = parse(WITH_SCHEMA).expect("metadata");
        let document = SkillDocument {
            metadata: metadata.clone(),
            body: "Body".to_string(),
        };

        let reparsed = parse(&compose(&document)).expect("round trip");

        assert_eq!(reparsed.config_schema_block, metadata.config_schema_block);
        assert_eq!(
            reparsed
                .config_schema()
                .expect("declared")
                .expect("supported")
                .hash,
            metadata
                .config_schema()
                .expect("declared")
                .expect("supported")
                .hash
        );
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

    /// `config_schema` and `delegation` were added by separate branches, each emitting its own
    /// indented block from the same `compose` format string. Only a document declaring both
    /// exercises the ordering between them, where one block's nested keys are the other's
    /// termination signal.
    #[test]
    fn a_skill_declaring_both_a_schema_and_a_delegation_contract_round_trips_both() {
        let source = "---\nid: configured-utility\nname: Configured Utility\ndescription: Both blocks\ncategory: test\nversion: 1.0.0\ntype: utility\ndelivery: on-demand\ntriggers:\n  - configured\naliases:\n  - cfg\ndelegation:\n  tools:\n    - file-read\n  max_rounds: 4\nconfig_schema:\n  properties:\n    endpoint:\n      type: string\n      default: https://example.com\n---\n\nBody";

        let metadata = parse(source).expect("metadata");
        assert!(metadata.is_configurable());
        assert_eq!(
            metadata.delegation.raw().expect("declared").tools,
            vec!["file-read"]
        );
        // The schema's nested keys must not have been swallowed by the delegation block that
        // precedes them, nor its own nested keys read as top-level frontmatter.
        assert_eq!(metadata.triggers, vec!["configured"]);
        assert_eq!(metadata.aliases[0].as_str(), "cfg");

        let composed = compose(&SkillDocument {
            metadata: metadata.clone(),
            body: "Body".to_string(),
        });

        assert_eq!(parse(&composed).expect("round trip"), metadata);
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
