use chrono::{DateTime, SecondsFormat, Utc};

use crate::contexts::personalization::application::PersonalizationApplicationError;
use crate::contexts::personalization::domain::{
    AgentId, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, SessionId, WorkspaceKey,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Bumped only when the on-disk shape changes in a way an older reader cannot handle. A file
/// declaring an unknown version is quarantined rather than guessed at.
pub(crate) const MEMORY_SCHEMA_VERSION: u32 = 2;

const DELIMITER: &str = "---";

/// Lines read from a file's head before giving up on finding the closing delimiter. Classification
/// runs over the whole directory, so it must not pull memory bodies into memory to decide what a
/// file is.
const MAX_FRONTMATTER_LINES: usize = 40;

/// What a file claims to be, decided from its header alone.
///
/// The filename cannot answer this: a v1 memory's name was derived from its display name, and
/// names like `use-pnpm` are also perfectly valid memory ids. The declared schema version is the
/// only signal that distinguishes the two formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentKind {
    /// Declares the current schema version. Whether it is *valid* is for `parse` to say.
    V2,
    /// Has frontmatter but does not declare this schema version — a pre-v2 memory, or one written
    /// by a newer build. Either way it must be migrated or quarantined, never activated.
    Legacy,
    /// No terminated frontmatter block at all. The v1 format always had one, so this is neither a
    /// legacy memory nor a readable v2 file.
    Unreadable,
}

/// Classifies a file from a bounded read of its head.
pub(crate) fn peek_kind(head: &str) -> DocumentKind {
    let normalized = normalize_body(head);
    let Some(rest) = normalized
        .strip_prefix(DELIMITER)
        .and_then(|rest| rest.strip_prefix('\n'))
    else {
        return DocumentKind::Unreadable;
    };
    let header = match rest.split_once("\n---") {
        Some((header, _)) => header,
        // The header window ran out before the closing delimiter. Treating that as legacy would
        // hand a truncated v2 file to the migration, so it stays unreadable.
        None => return DocumentKind::Unreadable,
    };
    for line in header.lines().take(MAX_FRONTMATTER_LINES) {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "schema_version" {
                return if value.trim() == MEMORY_SCHEMA_VERSION.to_string() {
                    DocumentKind::V2
                } else {
                    DocumentKind::Legacy
                };
            }
        }
    }
    DocumentKind::Legacy
}

fn malformed(reason: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(format!("memory file is malformed: {reason}"))
}

/// SHA-256 of the body, recorded in the frontmatter.
///
/// Its job is to make a torn write detectable. `create` opens the final path with create-new and
/// then writes; a crash between those leaves a file whose body does not match this hash, and
/// enumeration classifies it malformed instead of activating half a memory.
/// Line endings are normalized to LF before a body is hashed or written.
///
/// Without this the hash would depend on how an editor happened to save the file, and a memory
/// written on Windows would read as torn everywhere else. Normalizing once, at the boundary, keeps
/// the in-memory record and the file byte-identical in the only respect the hash cares about.
pub(crate) fn normalize_body(content: &str) -> String {
    content.replace("\r\n", "\n")
}

/// Re-exported from the domain: the SQLite projection needs the same fingerprint, and the
/// application layer that writes it may not reach into this module.
pub(crate) use crate::contexts::personalization::domain::content_hash;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| malformed(format!("unreadable timestamp {value:?}: {error}")))
}

/// User-supplied text is JSON-encoded so a name containing a colon, a quote, or a newline cannot
/// forge another frontmatter field. Encoding is cheaper than validating the many characters a
/// bare-token format would have to forbid, and it keeps the file valid YAML.
fn encode_text(value: &str) -> String {
    serde_json::Value::String(value.to_string()).to_string()
}

fn decode_text(value: &str) -> Result<String> {
    if !value.starts_with('"') {
        // Tolerated for hand-written files, which is a supported way to seed a memory.
        return Ok(value.to_string());
    }
    serde_json::from_str::<String>(value)
        .map_err(|error| malformed(format!("unreadable text: {error}")))
}

fn encode_audience(audience: &MemoryAudience) -> String {
    match audience {
        MemoryAudience::AllAgents => "all_agents".to_string(),
        MemoryAudience::SelectedAgents { agent_ids } => {
            let ids: Vec<&str> = agent_ids.iter().map(AgentId::as_str).collect();
            // A comma is impossible inside an AgentId, so this needs no escaping.
            format!("selected:{}", ids.join(","))
        }
    }
}

fn decode_audience(value: &str) -> Result<MemoryAudience> {
    if value == "all_agents" {
        return Ok(MemoryAudience::AllAgents);
    }
    let Some(list) = value.strip_prefix("selected:") else {
        return Err(malformed(format!("unknown audience {value:?}")));
    };
    let mut agent_ids = Vec::new();
    for id in list.split(',').filter(|id| !id.is_empty()) {
        agent_ids.push(AgentId::parse(id)?);
    }
    Ok(MemoryAudience::SelectedAgents { agent_ids })
}

/// Serializes a record to its authoritative on-disk form.
pub(crate) fn compose(record: &MemoryRecord) -> String {
    let mut lines = vec![
        DELIMITER.to_string(),
        format!("schema_version: {MEMORY_SCHEMA_VERSION}"),
        format!("id: {}", record.id),
        format!("name: {}", encode_text(&record.name)),
        format!("description: {}", encode_text(&record.description)),
        format!("memory_type: {}", record.memory_type.as_str()),
        format!("scope_kind: {}", record.scope.kind_str()),
    ];
    if let Some(workspace_key) = record.scope.workspace_key() {
        lines.push(format!("workspace_key: {workspace_key}"));
    }
    lines.push(format!("audience: {}", encode_audience(&record.audience)));
    lines.push(format!("status: {}", record.status.as_str()));
    lines.push(format!("source: {}", record.source.as_str()));
    if let Some(agent_id) = record.provenance.source_agent_id.as_ref() {
        lines.push(format!("source_agent_id: {agent_id}"));
    }
    if let Some(session_id) = record.provenance.source_session_id.as_ref() {
        lines.push(format!("source_session_id: {session_id}"));
    }
    if let Some(message_id) = record.provenance.source_message_id.as_ref() {
        lines.push(format!("source_message_id: {}", encode_text(message_id)));
    }
    if let Some(workspace_key) = record.provenance.source_workspace_key.as_ref() {
        lines.push(format!("source_workspace_key: {workspace_key}"));
    }
    lines.push(format!("sensitivity: {}", record.sensitivity.as_str()));
    lines.push(format!("revision: {}", record.revision));
    lines.push(format!("created_at: {}", timestamp(record.created_at)));
    lines.push(format!("updated_at: {}", timestamp(record.updated_at)));
    if let Some(verified_at) = record.verified_at {
        lines.push(format!("verified_at: {}", timestamp(verified_at)));
    }
    if let Some(last_used_at) = record.last_used_at {
        lines.push(format!("last_used_at: {}", timestamp(last_used_at)));
    }
    lines.push(format!("use_count: {}", record.use_count));
    let body = normalize_body(&record.content);
    lines.push(format!("content_hash: {}", content_hash(&body)));
    lines.push(DELIMITER.to_string());

    // Exactly one blank line between the closing delimiter and the body, and nothing appended
    // after it. The hash covers the body byte for byte, so composing must not add a trailing
    // newline that parsing would then have to guess whether to strip.
    format!("{}\n\n{body}", lines.join("\n"))
}

struct Frontmatter {
    fields: std::collections::BTreeMap<String, String>,
}

impl Frontmatter {
    fn required(&self, key: &str) -> Result<&str> {
        self.fields
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| malformed(format!("missing required field {key:?}")))
    }

    fn optional(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

fn split_document(raw: &str) -> Result<(Frontmatter, String)> {
    let normalized = normalize_body(raw);
    let rest = normalized
        .strip_prefix(DELIMITER)
        .and_then(|rest| rest.strip_prefix('\n'))
        .ok_or_else(|| malformed("file does not open with a frontmatter delimiter"))?;
    // The first closing delimiter is the header's: the header is written before the body, and
    // every header value is a single line, so a `---` inside the body cannot be reached first.
    let (header, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| malformed("frontmatter is not terminated"))?;
    // Exactly one blank line, not "as many as there are": a body that legitimately starts with a
    // blank line must survive the round trip, or its hash will not match.
    let body = body
        .strip_prefix('\n')
        .ok_or_else(|| malformed("the body is not separated from the frontmatter by a blank line"))?
        .to_string();

    let mut fields = std::collections::BTreeMap::new();
    for line in header.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line.split_once(':').ok_or_else(|| {
            malformed(format!("frontmatter line {line:?} is not a key/value pair"))
        })?;
        // Last wins would let a duplicated key override a checked one; refusing is the only safe
        // reading of a file that declares a field twice.
        if fields
            .insert(key.trim().to_string(), value.trim().to_string())
            .is_some()
        {
            return Err(malformed(format!("frontmatter declares {key:?} twice")));
        }
    }
    Ok((Frontmatter { fields }, body))
}

/// Rebuilds a record from its file, refusing anything the domain would not accept.
///
/// Every failure here means "quarantine", never "activate with defaults": a file this build cannot
/// read in full is a file whose scope and audience are unknown, and an unknown scope must not
/// resolve to global.
pub(crate) fn parse(raw: &str) -> Result<MemoryRecord> {
    let (frontmatter, body) = split_document(raw)?;

    let schema_version: u32 = frontmatter
        .required("schema_version")?
        .parse()
        .map_err(|_| malformed("schema_version is not a number"))?;
    if schema_version != MEMORY_SCHEMA_VERSION {
        return Err(malformed(format!(
            "unsupported schema_version {schema_version}"
        )));
    }

    let workspace_key = frontmatter
        .optional("workspace_key")
        .map(WorkspaceKey::parse)
        .transpose()?;
    let scope =
        MemoryScope::from_parts(frontmatter.required("scope_kind")?, workspace_key.as_ref())?;

    let record = MemoryRecord {
        id: MemoryId::parse(frontmatter.required("id")?)?,
        name: decode_text(frontmatter.required("name")?)?,
        description: frontmatter
            .optional("description")
            .map(decode_text)
            .transpose()?
            .unwrap_or_default(),
        memory_type: MemoryType::parse(frontmatter.required("memory_type")?)?,
        content: body,
        scope,
        audience: decode_audience(frontmatter.required("audience")?)?,
        status: MemoryStatus::parse(frontmatter.required("status")?)?,
        source: MemorySource::parse(frontmatter.required("source")?)?,
        provenance: MemoryProvenance {
            source_agent_id: frontmatter
                .optional("source_agent_id")
                .map(AgentId::parse)
                .transpose()?,
            source_session_id: frontmatter
                .optional("source_session_id")
                .map(SessionId::parse)
                .transpose()?,
            source_message_id: frontmatter
                .optional("source_message_id")
                .map(decode_text)
                .transpose()?,
            source_workspace_key: frontmatter
                .optional("source_workspace_key")
                .map(WorkspaceKey::parse)
                .transpose()?,
        },
        sensitivity: frontmatter
            .optional("sensitivity")
            .map(MemorySensitivity::parse)
            .transpose()?
            .unwrap_or(MemorySensitivity::Normal),
        revision: frontmatter
            .required("revision")?
            .parse()
            .map_err(|_| malformed("revision is not a number"))?,
        created_at: parse_timestamp(frontmatter.required("created_at")?)?,
        updated_at: parse_timestamp(frontmatter.required("updated_at")?)?,
        verified_at: frontmatter
            .optional("verified_at")
            .map(parse_timestamp)
            .transpose()?,
        last_used_at: frontmatter
            .optional("last_used_at")
            .map(parse_timestamp)
            .transpose()?,
        use_count: frontmatter
            .optional("use_count")
            .map(str::parse::<u64>)
            .transpose()
            .map_err(|_| malformed("use_count is not a number"))?
            .unwrap_or_default(),
    };

    let declared_hash = frontmatter.required("content_hash")?;
    let actual_hash = content_hash(&record.content);
    if declared_hash != actual_hash {
        return Err(malformed(
            "content_hash does not match the body; the file is torn or was edited without \
             updating its hash",
        ));
    }

    record.validate()?;
    Ok(record)
}
