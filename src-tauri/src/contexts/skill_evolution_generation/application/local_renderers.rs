use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::{
    application::sha256_bytes,
    domain::{GeneratedArtifactKind, RenderedGenerationArtifactV1, StructuredDraftV1},
};

pub(crate) const GENERATION_RENDERER_VERSION_V1: &str = "skill-generation-renderer-v1";
const MAX_LEARN_BLOCK_BYTES_V1: usize = 8 * 1024;
const MAX_EXACT_PATCH_BYTES_V1: usize = 16 * 1024;
const MAX_SKILL_BYTES_V1: usize = 12 * 1024;
const MAX_SKILL_BODY_BYTES_V1: usize = 4 * 1024;
const TARGET_SKILL_BODY_CHARACTERS_V1: usize = 2_000;

pub(crate) struct GenerationRenderRequestV1<'a> {
    pub(crate) artifact_id: &'a str,
    pub(crate) expected_kind: GeneratedArtifactKind,
    pub(crate) draft: &'a StructuredDraftV1,
    pub(crate) allowed_built_in_tools: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationRenderError {
    InvalidInput,
    KindMismatch,
    ForbiddenContent,
    UnsupportedDependency,
    Oversized,
    Failed,
}

pub(crate) fn render_generation_artifact(
    request: &GenerationRenderRequestV1<'_>,
) -> Result<RenderedGenerationArtifactV1, GenerationRenderError> {
    if request.artifact_id.trim().is_empty() {
        return Err(GenerationRenderError::InvalidInput);
    }
    let (kind, media_type, content) = match request.draft {
        StructuredDraftV1::OverlayLearnBlock { guidance } => (
            GeneratedArtifactKind::OverlayLearnBlock,
            "text/markdown",
            render_learn_block(guidance)?,
        ),
        StructuredDraftV1::OverlayExactPatch {
            old_string,
            new_string,
            replace_all,
        } => (
            GeneratedArtifactKind::OverlayExactPatch,
            "text/markdown",
            render_exact_patch(old_string, new_string, *replace_all)?,
        ),
        StructuredDraftV1::NewSkill {
            candidate_id,
            name,
            description,
            skill_type,
            version,
            built_in_tools,
            instructions,
        } => (
            GeneratedArtifactKind::NewSkill,
            "text/markdown",
            render_new_skill(
                &NewSkillFields {
                    candidate_id,
                    name,
                    description,
                    skill_type,
                    version,
                    built_in_tools,
                    instructions,
                },
                request.allowed_built_in_tools,
            )?,
        ),
    };
    if kind != request.expected_kind {
        return Err(GenerationRenderError::KindMismatch);
    }
    let size_bytes = u32::try_from(content.len()).map_err(|_| GenerationRenderError::Oversized)?;
    let content_hash = sha256_bytes(content.as_bytes());
    Ok(RenderedGenerationArtifactV1 {
        artifact_id: request.artifact_id.into(),
        artifact_kind: kind,
        renderer_version: GENERATION_RENDERER_VERSION_V1.into(),
        media_type: media_type.into(),
        content,
        size_bytes,
        content_hash,
    })
}

fn render_learn_block(guidance: &str) -> Result<String, GenerationRenderError> {
    let guidance = normalize_and_validate(guidance)?;
    let content = format!("## Learned guidance\n\n{}\n", escape_markdown(&guidance));
    bounded(content, MAX_LEARN_BLOCK_BYTES_V1)
}

fn render_exact_patch(
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, GenerationRenderError> {
    if replace_all {
        return Err(GenerationRenderError::InvalidInput);
    }
    let old_string = normalize_and_validate(old_string)?;
    let new_string = normalize_and_validate(new_string)?;
    if old_string == new_string || old_string.len() + new_string.len() > MAX_EXACT_PATCH_BYTES_V1 {
        return Err(GenerationRenderError::Oversized);
    }
    let content = format!(
        "## Exact patch\n\nReplace all: `false`\n\n### Current text\n\n{}\n\n### Replacement text\n\n{}\n",
        markdown_quote(&old_string),
        markdown_quote(&new_string),
    );
    bounded(content, MAX_EXACT_PATCH_BYTES_V1)
}

struct NewSkillFields<'a> {
    candidate_id: &'a str,
    name: &'a str,
    description: &'a str,
    skill_type: &'a str,
    version: &'a str,
    built_in_tools: &'a [String],
    instructions: &'a str,
}

fn render_new_skill(
    fields: &NewSkillFields<'_>,
    allowed_tools: &BTreeSet<String>,
) -> Result<String, GenerationRenderError> {
    if !valid_kebab_id(fields.candidate_id)
        || !matches!(fields.skill_type, "role" | "utility")
        || fields.name.trim().is_empty()
        || fields.description.trim().is_empty()
        || fields.description.chars().count() > 500
        || !valid_version(fields.version)
    {
        return Err(GenerationRenderError::InvalidInput);
    }
    let tools: BTreeSet<_> = fields.built_in_tools.iter().collect();
    if tools.len() != fields.built_in_tools.len()
        || tools.iter().any(|tool| !allowed_tools.contains(*tool))
    {
        return Err(GenerationRenderError::UnsupportedDependency);
    }
    let instructions = normalize_and_validate(fields.instructions)?;
    if instructions.len() > MAX_SKILL_BODY_BYTES_V1
        || instructions.chars().count() > TARGET_SKILL_BODY_CHARACTERS_V1
    {
        return Err(GenerationRenderError::Oversized);
    }
    let mut content = format!(
        "---\nid: {}\nname: {}\ndescription: {}\ncategory: generated\nversion: {}\ntype: {}\n",
        fields.candidate_id,
        yaml_string(fields.name)?,
        yaml_string(fields.description)?,
        yaml_string(fields.version)?,
        fields.skill_type,
    );
    if !tools.is_empty() {
        content.push_str("allowed-tools:\n");
        for tool in tools {
            content.push_str(&format!("  - {}\n", yaml_string(tool)?));
        }
    }
    content.push_str("---\n\n");
    content.push_str(&escape_markdown(&instructions));
    content.push('\n');
    bounded(content, MAX_SKILL_BYTES_V1)
}

fn normalize_and_validate(value: &str) -> Result<String, GenerationRenderError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let lower = normalized.to_ascii_lowercase();
    let forbidden = [
        "<!--",
        "<script",
        "<iframe",
        "javascript:",
        "#! /",
        "#!/",
        "curl http",
        "wget http",
        "powershell -enc",
        "rm -rf",
        "scripts/",
        "tools/",
        "references/",
        "templates/",
        "assets/",
        "config/",
        "api_key",
        "api-key",
        "password:",
        "secret:",
        "access_token",
    ];
    if normalized.trim().is_empty()
        || normalized.contains('\0')
        || forbidden.iter().any(|pattern| lower.contains(pattern))
        || contains_raw_html(&normalized)
    {
        return Err(GenerationRenderError::ForbiddenContent);
    }
    Ok(normalized.trim().into())
}

fn contains_raw_html(value: &str) -> bool {
    value.as_bytes().windows(2).any(|pair| {
        pair[0] == b'<' && (pair[1].is_ascii_alphabetic() || pair[1] == b'/' || pair[1] == b'!')
    })
}

fn escape_markdown(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('`', "\\`")
}

fn markdown_quote(value: &str) -> String {
    value
        .lines()
        .map(|line| format!("> {}", escape_markdown(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn yaml_string(value: &str) -> Result<String, GenerationRenderError> {
    serde_json::to_string(value).map_err(|_| GenerationRenderError::Failed)
}

fn valid_kebab_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn bounded(content: String, maximum: usize) -> Result<String, GenerationRenderError> {
    if content.len() > maximum {
        Err(GenerationRenderError::Oversized)
    } else {
        Ok(content)
    }
}
