use crate::contexts::agent_runtime::application::AgentRuntimeApplicationError;
use crate::contexts::agent_runtime::domain::{
    compose_memory_document, parse_memory_document, MemoryDocument, MemoryMetadata,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// The index is not a memory. It is excluded from every scan so it never becomes a candidate for
/// injection, selection, listing, or deletion.
pub(crate) const INDEX_FILE_NAME: &str = "MEMORY.md";

const MEMORY_DIRECTORY_NAME: &str = "memory";
const MEMORY_EXTENSION: &str = "md";

/// Upper bound on files considered by one scan. The pool is unbounded on disk; every consumer of a
/// scan is bounded, so the bound belongs here rather than being re-derived at each call site.
const MAX_SCANNED_FILES: usize = 200;

/// Lines read from a file before giving up on finding the closing frontmatter delimiter. A scan
/// reads headers for the whole directory on paths that run inside a generation, so it must not
/// pull memory bodies into memory to learn their descriptions.
const MAX_FRONTMATTER_LINES: usize = 30;

/// One memory's frontmatter plus the facts only the filesystem knows. `modified_at` is what
/// ordering uses: a memory the model has just corrected must count as the most recent one, which
/// its `created` frontmatter cannot express.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryHeader {
    pub(crate) relative_path: String,
    pub(crate) metadata: MemoryMetadata,
    pub(crate) modified_at: SystemTime,
}

/// Directory-backed memory store (`migrate-agent-memory-to-file-store`). The directory is
/// authoritative: the index is derived from it, and reconciliation resolves against it, so a file
/// added or removed outside the application converges without user action.
#[derive(Clone)]
pub(crate) struct FileAgentMemoryStore {
    root: PathBuf,
}

impl FileAgentMemoryStore {
    /// `data_root` is the application data directory — the same one `VANEHUB_APP_DATA_DIR`
    /// overrides, reached here through the database's own parent exactly as artifact blob storage
    /// reaches it, so the override applies without this module reading the environment itself.
    pub(crate) fn new(data_root: &Path) -> Result<Self, AgentRuntimeApplicationError> {
        let root = data_root.join(MEMORY_DIRECTORY_NAME);
        fs::create_dir_all(&root).map_err(|error| {
            memory_error(format!(
                "Memory directory {} is unavailable: {error}",
                root.display()
            ))
        })?;
        Ok(Self { root })
    }

    // Wired by task 4.4, which scopes the generic file tools to this directory.
    #[allow(dead_code)]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the directory has been initialized. The index's presence is the marker: it is
    /// legitimate directory content rather than a hidden sentinel, and it survives the user
    /// deleting every memory — which must not cause migration to resurrect them from the rows.
    pub(crate) fn has_index(&self) -> bool {
        self.root.join(INDEX_FILE_NAME).is_file()
    }

    /// Every memory in the directory, newest-modified first, capped at [`MAX_SCANNED_FILES`].
    ///
    /// A file whose frontmatter will not parse is skipped rather than failing the scan. This is
    /// what keeps a generation, an injection, or the management view alive when the directory —
    /// which is host-level and therefore shared across git worktrees — holds a file this build
    /// cannot read.
    pub(crate) fn scan(&self) -> Result<Vec<MemoryHeader>, AgentRuntimeApplicationError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            // A directory that has not been created yet is an empty pool, not a failure.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(memory_error(format!(
                    "Memory directory {} cannot be read: {error}",
                    self.root.display()
                )))
            }
        };

        let mut headers = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_memory_file(&path) {
                continue;
            }
            let Ok(file_metadata) = entry.metadata() else {
                continue;
            };
            let modified_at = file_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(frontmatter) = read_frontmatter_region(&path) else {
                continue;
            };
            let Ok(document) = parse_memory_document(&frontmatter) else {
                continue;
            };
            headers.push(MemoryHeader {
                relative_path: file_name.to_string(),
                metadata: document.metadata,
                modified_at,
            });
        }

        headers.sort_by(|left, right| {
            right
                .modified_at
                .cmp(&left.modified_at)
                // Ties are real: a migration writes many files within one filesystem timestamp
                // tick, and an unstable order there would reshuffle the index on every scan.
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });
        headers.truncate(MAX_SCANNED_FILES);
        Ok(headers)
    }

    // Wired by task 6.2 (management read path) and task 5.4 (applying an update action). Exercised
    // by this module's tests and by the migration tests until then.
    #[allow(dead_code)]
    pub(crate) fn read(
        &self,
        relative_path: &str,
    ) -> Result<MemoryDocument, AgentRuntimeApplicationError> {
        let path = self.resolve(relative_path)?;
        let content = fs::read_to_string(&path).map_err(|error| {
            memory_error(format!("Memory {relative_path} cannot be read: {error}"))
        })?;
        parse_memory_document(&content)
            .map_err(|error| memory_error(format!("Memory {relative_path} is malformed: {error}")))
    }

    /// Writes a memory and returns its directory-relative path. Saving over an existing name
    /// replaces that file rather than creating a second memory for the same name.
    pub(crate) fn write(
        &self,
        document: &MemoryDocument,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let relative_path = document.metadata.file_name();
        let path = self.resolve(&relative_path)?;
        fs::write(&path, compose_memory_document(document)).map_err(|error| {
            memory_error(format!("Memory {relative_path} cannot be written: {error}"))
        })?;
        Ok(relative_path)
    }

    // Wired by task 6.2 (`delete_agent_memory`).
    #[allow(dead_code)]
    pub(crate) fn delete(&self, relative_path: &str) -> Result<(), AgentRuntimeApplicationError> {
        let path = self.resolve(relative_path)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            // Deleting a memory that is already gone is the caller's desired end state.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(memory_error(format!(
                "Memory {relative_path} cannot be deleted: {error}"
            ))),
        }
    }

    // Wired by task 6.2 (`reset_agent_memories`).
    #[allow(dead_code)]
    pub(crate) fn delete_all(&self) -> Result<(), AgentRuntimeApplicationError> {
        for header in self.scan()? {
            self.delete(&header.relative_path)?;
        }
        self.write_index(&[])
    }

    /// Rebuilds `MEMORY.md` from a directory scan and returns the entry count. The index is derived
    /// state, never independently maintained, so an index line pointing at a missing file and a
    /// memory file with no index line both converge on the next call without special-case repair.
    pub(crate) fn reconcile_index(&self) -> Result<usize, AgentRuntimeApplicationError> {
        let headers = self.scan()?;
        self.write_index(&headers)?;
        Ok(headers.len())
    }

    fn write_index(&self, headers: &[MemoryHeader]) -> Result<(), AgentRuntimeApplicationError> {
        let path = self.root.join(INDEX_FILE_NAME);
        fs::write(&path, render_index(headers))
            .map_err(|error| memory_error(format!("Memory index cannot be written: {error}")))
    }

    /// Resolves a directory-relative memory path, rejecting anything that is not a single plain
    /// file name inside the memory directory.
    ///
    /// The directory is flat by design, so a legitimate path is always one `Normal` component. That
    /// makes traversal impossible by construction rather than by pattern-matching `..`, and leaves
    /// only the symlink case, which the canonicalized prefix check below covers for files that
    /// already exist.
    fn resolve(&self, relative_path: &str) -> Result<PathBuf, AgentRuntimeApplicationError> {
        let candidate = Path::new(relative_path);
        let mut components = candidate.components();
        let Some(Component::Normal(name)) = components.next() else {
            return Err(memory_error(format!(
                "Memory path {relative_path} must be a plain file name."
            )));
        };
        if components.next().is_some() {
            return Err(memory_error(format!(
                "Memory path {relative_path} must not contain a directory separator."
            )));
        }
        let path = self.root.join(name);
        if !is_memory_file(&path) {
            return Err(memory_error(format!(
                "Memory path {relative_path} must name a .md file that is not the index."
            )));
        }
        // Only meaningful for a file that already exists; a new file cannot be canonicalized, and
        // its parent is the root we just joined from.
        if path.exists() {
            let canonical_root = self.root.canonicalize().map_err(|error| {
                memory_error(format!("Memory directory cannot be resolved: {error}"))
            })?;
            let canonical_path = path.canonicalize().map_err(|error| {
                memory_error(format!(
                    "Memory {relative_path} cannot be resolved: {error}"
                ))
            })?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(memory_error(format!(
                    "Memory path {relative_path} resolves outside the memory directory."
                )));
            }
        }
        Ok(path)
    }
}

fn is_memory_file(path: &Path) -> bool {
    if path.extension().and_then(|extension| extension.to_str()) != Some(MEMORY_EXTENSION) {
        return false;
    }
    path.file_name().and_then(|name| name.to_str()) != Some(INDEX_FILE_NAME)
}

/// Reads only as far as the closing frontmatter delimiter, then appends a placeholder body so the
/// shared parser — which requires a non-empty body — can validate the header without the file's
/// real body ever being read.
fn read_frontmatter_region(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let mut lines = Vec::new();
    let mut closed = false;
    for line in BufReader::new(file).lines().take(MAX_FRONTMATTER_LINES) {
        let line = line?;
        let is_delimiter = line.trim_end() == "---";
        lines.push(line);
        if is_delimiter && lines.len() > 1 {
            closed = true;
            break;
        }
    }
    if !closed {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frontmatter is not terminated within the header window",
        ));
    }
    lines.push(String::new());
    lines.push("(body not read)".to_string());
    Ok(lines.join("\n"))
}

fn render_index(headers: &[MemoryHeader]) -> String {
    let mut lines = vec!["# Memory index".to_string(), String::new()];
    for header in headers {
        lines.push(format!(
            "- [{}]({}) — {}",
            header.metadata.name, header.relative_path, header.metadata.description
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn memory_error(message: String) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Memory(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::MemoryType;
    use crate::test_support::TempDirectory;

    struct Fixture {
        _directory: TempDirectory,
        store: FileAgentMemoryStore,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let store = FileAgentMemoryStore::new(directory.path()).expect("memory store");
            Self {
                _directory: directory,
                store,
            }
        }

        fn save(&self, name: &str, description: &str, body: &str) -> String {
            let document = MemoryDocument::new(
                MemoryMetadata::new(name, description, Some(MemoryType::Project))
                    .expect("metadata"),
                body,
            )
            .expect("document");
            self.store.write(&document).expect("write")
        }

        fn write_raw(&self, file_name: &str, content: &str) {
            fs::write(self.store.root().join(file_name), content).expect("raw fixture");
        }

        fn index(&self) -> String {
            fs::read_to_string(self.store.root().join(INDEX_FILE_NAME)).expect("index")
        }
    }

    #[test]
    fn new_creates_the_directory_under_the_application_data_root() {
        let directory = TempDirectory::new("memory store creates root");
        let store = FileAgentMemoryStore::new(directory.path()).expect("memory store");

        assert!(store.root().is_dir());
        assert_eq!(store.root(), directory.path().join("memory"));
        // Idempotent: startup runs this on every launch.
        FileAgentMemoryStore::new(directory.path()).expect("second construction");
    }

    #[test]
    fn write_then_read_round_trips_and_saving_over_a_name_replaces_it() {
        let fixture = Fixture::new("memory store round trip");

        let path = fixture.save("user-role", "The user is a data scientist", "First body.");
        assert_eq!(path, "user-role.md");
        assert_eq!(fixture.store.read(&path).expect("read").body, "First body.");

        fixture.save("user-role", "The user is a data scientist", "Second body.");

        assert_eq!(fixture.store.scan().expect("scan").len(), 1);
        assert_eq!(
            fixture.store.read(&path).expect("read").body,
            "Second body."
        );
    }

    #[test]
    fn scan_skips_malformed_files_instead_of_failing() {
        // A generation, an injection, and the management view all run this scan. The directory is
        // host-level and shared across worktrees, so one unreadable file must not take them down.
        let fixture = Fixture::new("memory store skips malformed");
        fixture.save("good", "A good memory", "Body.");
        fixture.write_raw("no-frontmatter.md", "Just a body.\n");
        fixture.write_raw("unterminated.md", "---\nname: x\ndescription: y\n");
        fixture.write_raw("missing-name.md", "---\ndescription: y\n---\n\nBody.\n");
        fixture.write_raw(
            "not-markdown.txt",
            "---\nname: t\ndescription: y\n---\n\nBody.\n",
        );

        let headers = fixture.store.scan().expect("scan");

        let names = headers
            .iter()
            .map(|header| header.relative_path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["good.md"]);
    }

    #[test]
    fn scan_excludes_the_index_file() {
        let fixture = Fixture::new("memory store excludes index");
        fixture.save("kept", "A memory", "Body.");
        fixture.store.reconcile_index().expect("reconcile");

        let headers = fixture.store.scan().expect("scan");

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].relative_path, "kept.md");
    }

    #[test]
    fn reconcile_index_drops_a_line_whose_file_is_gone() {
        let fixture = Fixture::new("memory store index drops orphan");
        fixture.save("kept", "Kept memory", "Body.");
        let removed = fixture.save("removed", "Removed memory", "Body.");
        fixture.store.reconcile_index().expect("first reconcile");
        assert!(fixture.index().contains("removed.md"));

        fs::remove_file(fixture.store.root().join(&removed)).expect("out-of-band delete");
        let count = fixture.store.reconcile_index().expect("second reconcile");

        assert_eq!(count, 1);
        assert!(fixture.index().contains("kept.md"));
        assert!(!fixture.index().contains("removed.md"));
    }

    #[test]
    fn reconcile_index_adds_a_file_that_has_no_line() {
        let fixture = Fixture::new("memory store index adds unlisted");
        fixture.store.reconcile_index().expect("empty reconcile");
        assert!(!fixture.index().contains("added.md"));

        // Written directly on disk, the way the model's own file tools write it.
        fixture.write_raw(
            "added.md",
            "---\nname: added\ndescription: Added out of band\n---\n\nBody.\n",
        );
        let count = fixture.store.reconcile_index().expect("reconcile");

        assert_eq!(count, 1);
        assert!(fixture
            .index()
            .contains("- [added](added.md) — Added out of band"));
    }

    #[test]
    fn scan_orders_by_modification_time_with_a_stable_tie_break() {
        let fixture = Fixture::new("memory store ordering");
        fixture.save("older", "Older memory", "Body.");
        fixture.save("newer", "Newer memory", "Body.");
        // Filesystem timestamp granularity makes same-tick writes routine; without the name
        // tie-break the index would reshuffle on every scan.
        let ordered = fixture.store.scan().expect("scan");

        assert_eq!(ordered.len(), 2);
        let first = &ordered[0];
        let second = &ordered[1];
        assert!(
            first.modified_at > second.modified_at
                || (first.modified_at == second.modified_at
                    && first.relative_path < second.relative_path)
        );
    }

    #[test]
    fn traversal_and_nested_paths_are_rejected() {
        let fixture = Fixture::new("memory store rejects traversal");
        for rejected in [
            "../escape.md",
            "..\\escape.md",
            "nested/inner.md",
            "nested\\inner.md",
            "",
            ".",
            "..",
            "plain.txt",
            INDEX_FILE_NAME,
        ] {
            assert!(
                fixture.store.read(rejected).is_err(),
                "expected read of {rejected:?} to be rejected"
            );
            assert!(
                fixture.store.delete(rejected).is_err(),
                "expected delete of {rejected:?} to be rejected"
            );
        }
    }

    #[test]
    fn an_absolute_path_is_rejected() {
        let fixture = Fixture::new("memory store rejects absolute");
        let outside = TempDirectory::new("memory store outside");
        let absolute = outside.path().join("escape.md");
        fs::write(
            &absolute,
            "---\nname: escape\ndescription: d\n---\n\nBody.\n",
        )
        .expect("fixture");

        assert!(fixture.store.read(&absolute.to_string_lossy()).is_err());
        assert!(absolute.exists(), "the outside file must be untouched");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_memory_pointing_outside_the_directory_is_rejected() {
        // Windows needs elevation or developer mode to create symlinks, so this runs on unix only.
        // The traversal test above is what covers the same escape lexically on every platform.
        let fixture = Fixture::new("memory store rejects symlink");
        let outside = TempDirectory::new("memory store symlink target");
        let target = outside.path().join("target.md");
        fs::write(&target, "---\nname: target\ndescription: d\n---\n\nBody.\n").expect("fixture");
        std::os::unix::fs::symlink(&target, fixture.store.root().join("link.md")).expect("symlink");

        assert!(fixture.store.read("link.md").is_err());
        assert!(fixture.store.delete("link.md").is_err());
    }

    #[test]
    fn delete_removes_the_target_and_tolerates_an_already_absent_file() {
        let fixture = Fixture::new("memory store delete");
        fixture.save("keep", "Kept", "Body.");
        let doomed = fixture.save("doomed", "Doomed", "Body.");

        fixture.store.delete(&doomed).expect("delete");
        fixture
            .store
            .delete(&doomed)
            .expect("deleting an absent memory is the caller's desired end state");

        let remaining = fixture.store.scan().expect("scan");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].relative_path, "keep.md");
    }

    #[test]
    fn delete_all_empties_the_pool_and_the_index() {
        let fixture = Fixture::new("memory store delete all");
        fixture.save("first", "First", "Body.");
        fixture.save("second", "Second", "Body.");
        fixture.store.reconcile_index().expect("reconcile");

        fixture.store.delete_all().expect("delete all");

        assert!(fixture.store.scan().expect("scan").is_empty());
        assert!(!fixture.index().contains("first.md"));
        assert!(!fixture.index().contains("second.md"));
    }

    #[test]
    fn scan_reads_headers_without_pulling_bodies_into_memory() {
        // The header window is what keeps a directory-wide scan cheap on a path that runs inside a
        // generation. A body far past the window must still yield a usable header.
        let fixture = Fixture::new("memory store header window");
        let body = "padding line\n".repeat(5_000);
        fixture.write_raw(
            "large.md",
            &format!("---\nname: large\ndescription: A large memory\n---\n\n{body}"),
        );

        let headers = fixture.store.scan().expect("scan");

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].metadata.description, "A large memory");
    }

    #[test]
    fn a_frontmatter_block_longer_than_the_header_window_is_skipped() {
        let fixture = Fixture::new("memory store oversized frontmatter");
        let filler = (0..MAX_FRONTMATTER_LINES + 5)
            .map(|index| format!("key{index}: value"))
            .collect::<Vec<_>>()
            .join("\n");
        fixture.write_raw(
            "oversized.md",
            &format!("---\nname: oversized\ndescription: d\n{filler}\n---\n\nBody.\n"),
        );

        assert!(fixture.store.scan().expect("scan").is_empty());
    }
}
