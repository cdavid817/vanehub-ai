use super::AgentRuntimeDomainError;

/// A memory's closed type taxonomy (`migrate-agent-memory-to-file-store`). Content derivable from
/// the project's current state — code patterns, architecture, git history — belongs in none of
/// these and is not memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl MemoryType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Feedback => "feedback",
            Self::Project => "project",
            Self::Reference => "reference",
        }
    }

    /// Tolerant by contract: an absent or unrecognized `type` degrades to an untyped memory rather
    /// than rejecting the file, so migrated files (which are deliberately written untyped) and
    /// files written by a branch running a different contract stay readable.
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "user" => Some(Self::User),
            "feedback" => Some(Self::Feedback),
            "project" => Some(Self::Project),
            "reference" => Some(Self::Reference),
            _ => None,
        }
    }
}

const MAX_NAME_CHARACTERS: usize = 100;
const MAX_DESCRIPTION_CHARACTERS: usize = 300;

/// Windows treats these as device names regardless of extension, so `con.md` is not a creatable
/// file. The model picks memory names freely, and this project's primary target is Windows.
const RESERVED_DEVICE_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Frontmatter of one memory file. `name` doubles as the file stem, so the file path is the
/// memory's identity and this field only restates it for readability. Provenance is kept as raw
/// strings: mapping `source` onto the application's `MemorySource` is a boundary concern, and the
/// domain must not reject a file for carrying a provenance value it does not recognize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryMetadata {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) agent_id: Option<String>,
    pub(crate) folder: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) created_at: Option<String>,
    /// Row id this file was converted from. Present only on migrated files, and the sole basis for
    /// migration idempotence.
    pub(crate) migrated_from: Option<String>,
}

impl MemoryMetadata {
    pub(crate) fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        memory_type: Option<MemoryType>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self {
            name: validate_name(&name.into())?,
            description: validate_description(&description.into())?,
            memory_type,
            agent_id: None,
            folder: None,
            source: None,
            created_at: None,
            migrated_from: None,
        })
    }

    pub(crate) fn with_provenance(
        mut self,
        agent_id: Option<String>,
        folder: Option<String>,
        source: Option<String>,
        created_at: Option<String>,
    ) -> Self {
        self.agent_id = agent_id;
        self.folder = folder;
        self.source = source;
        self.created_at = created_at;
        self
    }

    pub(crate) fn with_migrated_from(mut self, row_id: impl Into<String>) -> Self {
        self.migrated_from = Some(row_id.into());
        self
    }

    /// File name this memory is stored under, relative to the memory directory.
    pub(crate) fn file_name(&self) -> String {
        format!("{}.md", self.name)
    }
}

/// A parsed memory file: validated frontmatter plus the body below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryDocument {
    pub(crate) metadata: MemoryMetadata,
    pub(crate) body: String,
}

impl MemoryDocument {
    pub(crate) fn new(
        metadata: MemoryMetadata,
        body: impl Into<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let body = body.into();
        if body.trim().is_empty() {
            return Err(AgentRuntimeDomainError::InvalidMemoryValue("body"));
        }
        Ok(Self { metadata, body })
    }
}

/// Serialize a memory back to file text. Round-trips with [`parse_memory_document`]; the store's
/// write path uses this so no other module hand-assembles frontmatter.
pub(crate) fn compose_memory_document(document: &MemoryDocument) -> String {
    let metadata = &document.metadata;
    let mut lines = vec![
        "---".to_string(),
        format!("name: {}", metadata.name),
        format!("description: {}", metadata.description),
    ];
    if let Some(memory_type) = metadata.memory_type {
        lines.push(format!("type: {}", memory_type.as_str()));
    }
    for (key, value) in [
        ("agent", metadata.agent_id.as_deref()),
        ("folder", metadata.folder.as_deref()),
        ("source", metadata.source.as_deref()),
        ("created", metadata.created_at.as_deref()),
        ("migrated_from", metadata.migrated_from.as_deref()),
    ] {
        if let Some(value) = value {
            lines.push(format!("{key}: {value}"));
        }
    }
    lines.push("---".to_string());
    format!("{}\n\n{}\n", lines.join("\n"), document.body.trim())
}

/// Parse a memory file. Unknown keys are ignored rather than rejected: the memory directory is
/// host-level and therefore shared across git worktrees, so a branch running a different contract
/// will write keys this build has never heard of, and refusing them would make the two branches
/// mutually destructive.
pub(crate) fn parse_memory_document(
    content: &str,
) -> Result<MemoryDocument, AgentRuntimeDomainError> {
    let normalized = content.replace("\r\n", "\n");
    let (frontmatter, remainder) = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .ok_or(AgentRuntimeDomainError::MemoryFrontmatterMissing)?;

    let metadata = parse_frontmatter(frontmatter)?;
    let body = remainder
        .split_once('\n')
        .map(|(_, body)| body)
        .unwrap_or_default();
    MemoryDocument::new(metadata, body.trim())
}

fn parse_frontmatter(frontmatter: &str) -> Result<MemoryMetadata, AgentRuntimeDomainError> {
    let mut name = None;
    let mut description = None;
    let mut memory_type = None;
    let mut agent_id = None;
    let mut folder = None;
    let mut source = None;
    let mut created_at = None;
    let mut migrated_from = None;

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
        match key.trim() {
            "name" => name = Some(value),
            "description" => description = Some(value),
            "type" => memory_type = MemoryType::parse(&value),
            "agent" => agent_id = Some(value),
            "folder" => folder = Some(value),
            "source" => source = Some(value),
            "created" => created_at = Some(value),
            "migrated_from" => migrated_from = Some(value),
            _ => {}
        }
    }

    let name = name.ok_or(AgentRuntimeDomainError::InvalidMemoryValue("name"))?;
    let description =
        description.ok_or(AgentRuntimeDomainError::InvalidMemoryValue("description"))?;
    let mut metadata = MemoryMetadata::new(name, description, memory_type)?
        .with_provenance(agent_id, folder, source, created_at);
    metadata.migrated_from = migrated_from;
    Ok(metadata)
}

/// A memory's name is also its file stem, so anything that cannot be a file name cannot be a name.
pub(crate) fn validate_name(raw: &str) -> Result<String, AgentRuntimeDomainError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue("name"));
    }
    if name.chars().count() > MAX_NAME_CHARACTERS {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue("name length"));
    }
    if name.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
            )
    }) {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue(
            "name characters",
        ));
    }
    // `..` anywhere is rejected rather than only as a whole segment: a name is a single file stem,
    // so there is no legitimate reason for it to appear at all, and the loose check is the one that
    // cannot be walked around.
    if name.contains("..") || name.starts_with('.') || name.ends_with('.') {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue("name dots"));
    }
    if RESERVED_DEVICE_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue(
            "name reserved on Windows",
        ));
    }
    Ok(name.to_string())
}

fn validate_description(raw: &str) -> Result<String, AgentRuntimeDomainError> {
    let description = raw.trim();
    if description.is_empty() {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue("description"));
    }
    if description.chars().count() > MAX_DESCRIPTION_CHARACTERS {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue(
            "description length",
        ));
    }
    // The description is a one-line index entry and a one-line manifest row. A newline in it would
    // silently split one memory into two rows in both surfaces.
    if description.chars().any(char::is_control) {
        return Err(AgentRuntimeDomainError::InvalidMemoryValue(
            "description characters",
        ));
    }
    Ok(description.to_string())
}
