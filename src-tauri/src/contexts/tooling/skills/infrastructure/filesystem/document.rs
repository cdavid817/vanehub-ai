use crate::contexts::tooling::skills::application::{SkillApplicationError, SkillDocument};
use crate::contexts::tooling::skills::domain::SkillMetadata;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

const MAX_IMPORT_FILES: usize = 512;
const MAX_IMPORT_DEPTH: usize = 16;
const MAX_IMPORT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SKILL_DOCUMENT_BYTES: u64 = 256 * 1024;

pub(super) fn compose(document: &SkillDocument) -> String {
    let triggers = document
        .metadata
        .triggers
        .iter()
        .map(|trigger| format!("  - {trigger}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nid: {}\nname: {}\ndescription: {}\ncategory: {}\nversion: {}\ntriggers:\n{}\n---\n\n# {}\n\n{}\n",
        document.metadata.id.as_str(),
        document.metadata.name,
        document.metadata.description,
        document.metadata.category,
        document.metadata.version,
        triggers,
        document.metadata.name,
        document.body.trim()
    )
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
    let mut in_triggers = false;
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if in_triggers && line.starts_with('-') {
            triggers.push(
                line.trim_start_matches('-')
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
            continue;
        }
        in_triggers = false;
        if line == "triggers:" {
            in_triggers = true;
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
            _ => {}
        }
    }
    SkillMetadata::new(id, name, description, category, version, triggers)
        .map_err(|error| validation_error(error.to_string()))
}

pub(super) fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(super) fn copy_directory(source: &Path, target: &Path) -> Result<(), SkillApplicationError> {
    let mut budget = ImportBudget::default();
    copy_directory_bounded(source, target, 0, &mut budget)
}

pub(super) fn read_import_document(path: &Path) -> Result<String, SkillApplicationError> {
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
        return Err(validation_error("Invalid Skill source: import depth exceeds 16"));
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
