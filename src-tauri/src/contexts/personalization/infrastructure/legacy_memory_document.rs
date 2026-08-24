//! Reader for the pre-governance memory file format.
//!
//! Deliberately a private copy of the v1 rule rather than a call into the context that still owns
//! the v1 writer. Personalization may not depend on `agent_runtime`, and the v1 format is frozen —
//! it gains no new writer — so there is nothing for the two readers to drift about. When the v1
//! store is removed, this stays: a migration has to keep reading a format after its writer is gone.
//!
//! Deliberately more permissive than v1's own parser. v1 refused a file whose name exceeded its
//! limit or whose description was missing, which made that file invisible rather than absent — the
//! text stayed on disk and no surface showed it. v2's bounds are wider, so this extracts what is
//! there and lets `MemoryRecord::validate` be the only gate. A file this cannot read at all is
//! quarantined, never dropped.

/// One v1 memory file, as far as it can be read.
///
/// Every field except `name` and `body` is optional because every one of them was optional in some
/// v1 file that exists in the wild: files written before a key was introduced, files written by a
/// branch running a different contract, and files a user wrote by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyDocument {
    pub(crate) name: String,
    pub(crate) description: String,
    /// The raw `type` value. Left as a string so mapping it onto the v2 taxonomy — including
    /// deciding that an unrecognized value becomes explicitly untyped — happens in one place.
    pub(crate) memory_type: Option<String>,
    pub(crate) agent_id: Option<String>,
    /// The raw workspace path v1 recorded. Not a workspace key: v1 stored the display path, and
    /// turning it into a stable key is the identity resolver's job.
    pub(crate) folder: Option<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) body: String,
}

/// Parses a v1 file, or reports that it cannot be read.
///
/// Returns `None` rather than an error type: every failure has the same consequence — the file is
/// quarantined with its bytes intact — so distinguishing them would add a taxonomy no caller acts
/// on. The reason is recorded on the journal entry instead.
pub(crate) fn parse_legacy_document(content: &str) -> Option<LegacyDocument> {
    // CRLF is normalized first so a file saved on Windows parses identically to the same file saved
    // anywhere else, exactly as v1 did.
    let normalized = content.replace("\r\n", "\n");
    let (frontmatter, remainder) = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))?;

    let mut fields = LegacyFields::default();
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        // Only the first colon separates key from value, so a Windows path in `folder` and a colon
        // inside a description both survive intact.
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        if value.is_empty() {
            continue;
        }
        // Unknown keys are ignored rather than rejected. The memory directory is host-level and
        // shared across worktrees, so a branch running a different contract will have written keys
        // this build has never heard of, and refusing them would strand those files.
        match key.trim() {
            "name" => fields.name = Some(value),
            "description" => fields.description = Some(value),
            "type" => fields.memory_type = Some(value),
            "agent" => fields.agent_id = Some(value),
            "folder" => fields.folder = Some(value),
            "created" => fields.created_at = Some(value),
            _ => {}
        }
    }

    let body = remainder
        .split_once('\n')
        .map(|(_, body)| body)
        .unwrap_or_default()
        .trim()
        .to_string();

    // The two things a memory cannot be reconstructed without. A file missing either is exactly the
    // file v1's own scan skipped, and it is quarantined rather than migrated with invented values.
    let name = fields.name.filter(|name| !name.trim().is_empty())?;
    if body.is_empty() {
        return None;
    }

    Some(LegacyDocument {
        name: name.trim().to_string(),
        // Absent under v1 meant unreadable; here it means empty, which v2 permits. Recovering the
        // body is worth more than the one line of metadata that is missing.
        description: fields.description.unwrap_or_default(),
        memory_type: fields.memory_type,
        agent_id: fields.agent_id,
        folder: fields.folder,
        created_at: fields.created_at,
        body,
    })
}

#[derive(Default)]
struct LegacyFields {
    name: Option<String>,
    description: Option<String>,
    memory_type: Option<String>,
    agent_id: Option<String>,
    folder: Option<String>,
    created_at: Option<String>,
}
