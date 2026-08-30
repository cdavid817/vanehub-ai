use crate::contexts::skill_evolution_curation::domain::*;

pub(crate) const CURATOR_DRAFT_POLICY_VERSION: &str = "curator-draft-policy-v1";
const MAX_LEARN_BYTES: usize = 8 * 1024;
const MAX_PATCH_BYTES: usize = 16 * 1024;
const MAX_METADATA_BYTES: usize = 2 * 1024;

pub(crate) fn validate_request_shape(
    request: &CuratorDraftRequestV1,
    binding: &CuratorDraftCandidateBinding,
) -> Result<(), &'static str> {
    if request.schema_version != 1 || request.candidate_id != binding.candidate_id {
        return Err("draft.invalid-request");
    }
    if request.expected_candidate_revision != binding.candidate_revision {
        return Err("draft.stale-candidate");
    }
    if request
        .target_skill_id
        .as_deref()
        .is_some_and(|value| value != binding.target_skill_id)
        || request
            .target_revision
            .as_deref()
            .is_some_and(|value| value != binding.target_revision)
        || request
            .overlay_scope
            .as_deref()
            .is_some_and(|value| value != binding.overlay_scope)
    {
        return Err("draft.target-override");
    }
    if request.overlay_scope.as_deref() == Some("system") {
        return Err("draft.system-scope-escalation");
    }
    if !request.supporting_files.is_empty() {
        return Err("draft.supporting-files-prohibited");
    }
    if !request.requested_permissions.is_empty() {
        return Err("draft.permission-expansion-prohibited");
    }
    if !request.commands.is_empty() {
        return Err("draft.commands-prohibited");
    }
    if request.direct_base_edit {
        return Err("draft.base-edit-prohibited");
    }
    if request.rationale.trim().is_empty()
        || request.rationale.len() > MAX_METADATA_BYTES
        || request.expected_effective_change.trim().is_empty()
        || request.expected_effective_change.len() > MAX_METADATA_BYTES
    {
        return Err("draft.metadata-invalid");
    }
    validate_mutation(&request.mutation)
}

fn validate_mutation(mutation: &CuratorDraftMutationInput) -> Result<(), &'static str> {
    match mutation {
        CuratorDraftMutationInput::LearnedGuidance { guidance } => {
            validate_text(guidance, MAX_LEARN_BYTES, true)?;
        }
        CuratorDraftMutationInput::ExactPatch {
            old_string,
            new_string,
            ..
        } => {
            if old_string.is_empty() || old_string == new_string {
                return Err("draft.exact-patch-mismatch");
            }
            if old_string.len() + new_string.len() > MAX_PATCH_BYTES {
                return Err("draft.size-limit");
            }
            validate_text(old_string, MAX_PATCH_BYTES, false)?;
            validate_text(new_string, MAX_PATCH_BYTES, false)?;
        }
    }
    Ok(())
}

fn validate_text(value: &str, maximum: usize, markdown: bool) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > maximum {
        return Err("draft.size-limit");
    }
    if value.chars().any(|character| {
        character == '\0' || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err("draft.utf8-or-control-invalid");
    }
    if markdown && !value.matches("```").count().is_multiple_of(2) {
        return Err("draft.markdown-invalid");
    }
    let lowercase = value.to_ascii_lowercase();
    if [
        "ignore previous instructions",
        "override the system message",
        "<script",
        "-----begin private key",
    ]
    .iter()
    .any(|pattern| lowercase.contains(pattern))
    {
        return Err("draft.injection-or-secret");
    }
    if [
        "password=",
        "password:",
        "api_key=",
        "api-key:",
        "access_token=",
        "client_secret=",
    ]
    .iter()
    .any(|pattern| lowercase.contains(pattern))
    {
        return Err("draft.injection-or-secret");
    }
    if [
        "allowed-tools:",
        "register tool",
        "permission expansion",
        "directly edit the base",
    ]
    .iter()
    .any(|pattern| lowercase.contains(pattern))
    {
        return Err("draft.capability-expansion-prohibited");
    }
    if has_executable_or_command_content(&lowercase) {
        return Err("draft.executable-content-prohibited");
    }
    Ok(())
}

fn has_executable_or_command_content(value: &str) -> bool {
    let executable = [
        ".sh", ".ps1", ".bat", ".cmd", ".exe", "#!/", "```bash", "```shell",
    ];
    executable.iter().any(|pattern| value.contains(pattern))
        || value.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("$ ")
                || line.starts_with("sudo ")
                || line.starts_with("rm -")
                || (line.contains("curl ") && line.contains("| sh"))
        })
}
