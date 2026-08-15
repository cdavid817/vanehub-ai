use super::session_queries::resolve_session_root;
use crate::contexts::workspaces::application::{
    FileSearchListing, FileSearchMatch, SessionWorkspaceContext,
    WorkspaceApplicationError as AppError,
};
use crate::contexts::workspaces::domain::CanonicalPathBoundary;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Deeper than the Documents tab limit: a docs folder is shallow, a source tree is not.
const SEARCH_DEPTH_LIMIT: usize = 10;
/// Ceiling on returned matches regardless of what the caller asks for.
const SEARCH_RESULT_CAP: usize = 50;
/// Stop after this many eligible files so a monorepo cannot stall the composer.
const SEARCH_SCAN_LIMIT: usize = 20_000;

const SCORE_EXACT: u32 = 100;
const SCORE_PREFIX: u32 = 80;
const SCORE_SUBSTRING: u32 = 60;
const SCORE_PATH: u32 = 40;

/// Vendored and generated trees skipped for mention candidates so the result budget is
/// spent on first-party files. Deliberately absent: `bin`, because Cargo treats `src/bin`
/// as real source. The Documents tab traversal does not consult this list.
const EXCLUDED_DIRECTORIES: &[&str] = &[
    "node_modules",
    "bower_components",
    "jspm_packages",
    "vendor",
    "dist",
    "build",
    "out",
    "target",
    "obj",
    "__pycache__",
    "venv",
    "site-packages",
    "coverage",
    "pods",
    "deriveddata",
];

/// Source and configuration extensions eligible as mention candidates. Kept next to
/// EXCLUDED_DIRECTORIES so both bounds are adjusted in one place.
const SOURCE_EXTENSIONS: &[&str] = &[
    // systems
    "rs",
    "go",
    "c",
    "h",
    "cc",
    "cpp",
    "cxx",
    "hpp",
    "hh",
    "cs",
    "swift",
    "kt",
    "kts",
    "java",
    "scala",
    "m",
    "mm",
    "zig", // scripting
    "py",
    "rb",
    "php",
    "pl",
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "psm1",
    "lua",
    "r",
    // web
    "js",
    "jsx",
    "ts",
    "tsx",
    "mjs",
    "cjs",
    "mts",
    "cts",
    "vue",
    "svelte",
    "astro",
    "html",
    "htm",
    "css",
    "scss",
    "sass",
    "less", // data and configuration
    "json",
    "jsonc",
    "yaml",
    "yml",
    "toml",
    "ini",
    "cfg",
    "conf",
    "xml",
    "properties",
    "gradle",
    "tf",
    "tfvars",
    "proto",
    "cmake", // query and notebook
    "sql",
    "graphql",
    "gql",
    "prisma",
    "ipynb", // prose
    "md",
    "markdown",
    "txt",
    "rst",
    "adoc",
];

/// Extensionless filenames worth referencing.
const SOURCE_FILENAMES: &[&str] = &[
    "dockerfile",
    "makefile",
    "rakefile",
    "gemfile",
    "procfile",
    "justfile",
    "vagrantfile",
    "brewfile",
];

struct ScoredMatch {
    score: u32,
    depth: usize,
    name: String,
    path: String,
}

pub(crate) fn search_session_files(
    conn: &Connection,
    session_id: &str,
    query: &str,
    max_results: usize,
) -> Result<FileSearchListing, AppError> {
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(FileSearchListing {
            context: SessionWorkspaceContext::unavailable("Session workspace is unavailable."),
            items: Vec::new(),
            truncated: false,
        });
    };
    let limit = max_results.clamp(1, SEARCH_RESULT_CAP);
    let normalized = query.trim().to_ascii_lowercase().replace('\\', "/");
    let (items, truncated) = walk_candidates(&root, &normalized, limit)?;
    Ok(FileSearchListing {
        context: SessionWorkspaceContext::available(
            root.file_name()
                .map(|name| name.to_string_lossy().to_string()),
        ),
        items,
        truncated,
    })
}

fn walk_candidates(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<(Vec<FileSearchMatch>, bool), AppError> {
    // Entries are canonicalized before the containment check, so the root must be too:
    // a short (8.3) or symlinked root would otherwise fail every check and return nothing.
    let canonical_root = root
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let root = canonical_root.as_path();
    let boundary = CanonicalPathBoundary::new(root);
    let mut queue = VecDeque::from([(root.to_path_buf(), 0usize)]);
    let mut visited: HashSet<PathBuf> = HashSet::from([root.to_path_buf()]);
    let mut scored: Vec<ScoredMatch> = Vec::new();
    let mut exact = 0usize;
    let mut scanned = 0usize;
    let mut truncated = false;

    while let Some((directory, depth)) = queue.pop_front() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            // An unreadable root is a real failure; an unreadable subdirectory is a
            // permission quirk that must not fail the whole search.
            Err(error) if depth == 0 => return Err(AppError::Storage(error.to_string())),
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let canonical = match entry.path().canonicalize() {
                Ok(value) if value.starts_with(root) => value,
                _ => continue,
            };
            if canonical.is_dir() {
                if is_excluded_directory(&name) {
                    continue;
                }
                if depth + 1 > SEARCH_DEPTH_LIMIT {
                    truncated = true;
                    continue;
                }
                if visited.insert(canonical.clone()) {
                    queue.push_back((canonical, depth + 1));
                }
                continue;
            }
            if !is_eligible_file(&canonical) {
                continue;
            }
            scanned += 1;
            if scanned > SEARCH_SCAN_LIMIT {
                truncated = true;
                break;
            }
            let relative = boundary.relative(&canonical).map_err(|_| {
                AppError::Validation("Path resolves outside the session root.".to_string())
            })?;
            let Some(score) = score_candidate(query, &name, &relative) else {
                continue;
            };
            if score == SCORE_EXACT {
                exact += 1;
            }
            scored.push(ScoredMatch {
                score,
                depth: relative.matches('/').count(),
                name,
                path: relative,
            });
        }
        // An empty query browses rather than searches, and breadth-first order already
        // yields the shallowest files first, so a full budget cannot be improved on.
        if query.is_empty() && scored.len() >= limit {
            truncated = true;
            break;
        }
        // A full budget of top-tier matches fixes the result set no matter what follows.
        if !query.is_empty() && exact >= limit {
            break;
        }
        if scanned > SEARCH_SCAN_LIMIT {
            truncated = true;
            break;
        }
    }

    scored.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.depth.cmp(&right.depth))
            .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });
    truncated |= scored.len() > limit;
    Ok((
        scored
            .into_iter()
            .take(limit)
            .map(|entry| FileSearchMatch {
                name: entry.name,
                path: entry.path,
            })
            .collect(),
        truncated,
    ))
}

fn is_excluded_directory(name: &str) -> bool {
    EXCLUDED_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str())
}

fn is_eligible_file(path: &Path) -> bool {
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        if SOURCE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()) {
            return true;
        }
    }
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| SOURCE_FILENAMES.contains(&name.to_ascii_lowercase().as_str()))
}

/// Scores a candidate against an already normalized (trimmed, lowercased, forward-slashed)
/// query. `None` excludes the candidate.
fn score_candidate(query: &str, name: &str, relative_path: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(0);
    }
    let name = name.to_ascii_lowercase();
    let relative = relative_path.to_ascii_lowercase();
    if query.contains('/') {
        return score_text(query, &relative)
            .or_else(|| segments_in_order(query, &relative).then_some(SCORE_PATH));
    }
    score_text(query, &name).or_else(|| relative.contains(query).then_some(SCORE_PATH))
}

fn score_text(query: &str, text: &str) -> Option<u32> {
    if text == query {
        Some(SCORE_EXACT)
    } else if text.starts_with(query) {
        Some(SCORE_PREFIX)
    } else if text.contains(query) {
        Some(SCORE_SUBSTRING)
    } else {
        None
    }
}

fn segments_in_order(query: &str, relative: &str) -> bool {
    let mut cursor = 0usize;
    for segment in query.split('/').filter(|segment| !segment.is_empty()) {
        match relative[cursor..].find(segment) {
            Some(offset) => cursor += offset + segment.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn write_file(root: &Path, relative: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory");
        }
        fs::write(&path, b"fixture").expect("fixture file");
    }

    fn paths(root: &Path, query: &str, limit: usize) -> Vec<String> {
        walk_candidates(root, query, limit)
            .expect("walk")
            .0
            .into_iter()
            .map(|entry| entry.path)
            .collect()
    }

    #[test]
    fn allowlist_admits_source_families_and_rejects_generated_artifacts() {
        for eligible in [
            "a.rs",
            "a.ts",
            "a.tsx",
            "a.py",
            "a.go",
            "a.java",
            "a.kt",
            "a.swift",
            "a.rb",
            "a.php",
            "a.sh",
            "a.ps1",
            "a.vue",
            "a.svelte",
            "a.css",
            "a.scss",
            "a.html",
            "a.json",
            "a.yaml",
            "a.toml",
            "a.sql",
            "a.proto",
            "a.tf",
            "a.md",
            "a.txt",
            "Dockerfile",
            "Makefile",
        ] {
            assert!(
                is_eligible_file(Path::new(eligible)),
                "expected {eligible} to be eligible"
            );
        }
        for rejected in [
            "a.png", "a.jpg", "a.pdf", "a.zip", "a.exe", "a.dll", "a.so", "a.wasm", "a.lock",
            "a.bin", "a",
        ] {
            assert!(
                !is_eligible_file(Path::new(rejected)),
                "expected {rejected} to be rejected"
            );
        }
    }

    #[test]
    fn allowlist_matching_ignores_extension_case() {
        assert!(is_eligible_file(Path::new("Component.TSX")));
        assert!(is_eligible_file(Path::new("DOCKERFILE")));
    }

    #[test]
    fn excluded_directories_are_never_descended_into() {
        let fixture = TempDirectory::new("mention-search-excluded");
        let root = fixture.path();
        write_file(root, "src/utils.rs");
        for excluded in [
            "node_modules/pkg/utils.rs",
            "dist/utils.rs",
            "build/utils.rs",
            "target/debug/utils.rs",
            "__pycache__/utils.rs",
            "vendor/utils.rs",
            "coverage/utils.rs",
        ] {
            write_file(root, excluded);
        }
        assert_eq!(paths(root, "utils", 50), vec!["src/utils.rs".to_string()]);
    }

    #[test]
    fn cargo_style_src_bin_stays_searchable() {
        let fixture = TempDirectory::new("mention-search-src-bin");
        let root = fixture.path();
        write_file(root, "src/bin/tool.rs");
        assert_eq!(paths(root, "tool", 50), vec!["src/bin/tool.rs".to_string()]);
    }

    #[test]
    fn dot_prefixed_entries_stay_excluded() {
        let fixture = TempDirectory::new("mention-search-dotfiles");
        let root = fixture.path();
        write_file(root, "src/keep.rs");
        write_file(root, ".git/hooks/keep.rs");
        write_file(root, ".venv/lib/keep.rs");
        assert_eq!(paths(root, "keep", 50), vec!["src/keep.rs".to_string()]);
    }

    #[test]
    fn ranking_orders_exact_then_prefix_then_substring_then_path() {
        let fixture = TempDirectory::new("mention-search-ranking");
        let root = fixture.path();
        write_file(root, "util.rs"); // no match: "utils" is not in "util.rs"
        write_file(root, "utils.rs"); // exact for "utils.rs", prefix for "utils"
        write_file(root, "my_utils.rs"); // substring
        write_file(root, "utils/helper.rs"); // path-only
        let ordered = paths(root, "utils.rs", 50);
        assert_eq!(ordered.first(), Some(&"utils.rs".to_string()));
        assert!(ordered.contains(&"my_utils.rs".to_string()));
        assert!(!ordered.contains(&"utils/helper.rs".to_string()));

        let ordered = paths(root, "utils", 50);
        assert_eq!(
            ordered,
            vec![
                "utils.rs".to_string(),
                "my_utils.rs".to_string(),
                "utils/helper.rs".to_string(),
            ]
        );
    }

    #[test]
    fn exact_filename_match_outranks_a_shallower_prefix_match() {
        let fixture = TempDirectory::new("mention-search-exact");
        let root = fixture.path();
        write_file(root, "shallow/main.rs.ts");
        write_file(root, "deep/nested/tree/main.rs");
        assert_eq!(
            paths(root, "main.rs", 50),
            vec![
                "deep/nested/tree/main.rs".to_string(),
                "shallow/main.rs.ts".to_string(),
            ]
        );
    }

    #[test]
    fn ties_break_on_depth_then_path_order() {
        let fixture = TempDirectory::new("mention-search-ties");
        let root = fixture.path();
        write_file(root, "b/target.rs");
        write_file(root, "a/target.rs");
        write_file(root, "deep/a/target.rs");
        assert_eq!(
            paths(root, "target.rs", 50),
            vec![
                "a/target.rs".to_string(),
                "b/target.rs".to_string(),
                "deep/a/target.rs".to_string(),
            ]
        );
    }

    #[test]
    fn path_separator_query_matches_the_relative_path() {
        let fixture = TempDirectory::new("mention-search-path-query");
        let root = fixture.path();
        write_file(root, "src/components/chat/ChatInputBox.tsx");
        write_file(root, "src/other/ChatInputBox.tsx");
        let ordered = paths(root, "chat/chatinputbox", 50);
        assert_eq!(
            ordered,
            vec!["src/components/chat/ChatInputBox.tsx".to_string()]
        );
    }

    #[test]
    fn backslash_query_is_normalized_to_forward_slashes() {
        let fixture = TempDirectory::new("mention-search-backslash");
        let root = fixture.path();
        write_file(root, "src/components/Widget.tsx");
        let normalized = "src\\components".to_ascii_lowercase().replace('\\', "/");
        assert_eq!(
            paths(root, &normalized, 50),
            vec!["src/components/Widget.tsx".to_string()]
        );
    }

    #[test]
    fn query_matching_ignores_case() {
        let fixture = TempDirectory::new("mention-search-case");
        let root = fixture.path();
        write_file(root, "src/ChatInputBox.tsx");
        assert_eq!(
            paths(root, "chatinputbox", 50),
            vec!["src/ChatInputBox.tsx".to_string()]
        );
    }

    #[test]
    fn empty_query_browses_shallowest_files_first() {
        let fixture = TempDirectory::new("mention-search-browse");
        let root = fixture.path();
        write_file(root, "deep/nested/leaf.rs");
        write_file(root, "top.rs");
        let ordered = paths(root, "", 50);
        assert_eq!(ordered.first(), Some(&"top.rs".to_string()));
    }

    #[test]
    fn result_count_is_capped_and_reports_truncation() {
        let fixture = TempDirectory::new("mention-search-cap");
        let root = fixture.path();
        for index in 0..12 {
            write_file(root, &format!("src/match_{index}.rs"));
        }
        let (items, truncated) = walk_candidates(root, "match", 5).expect("walk");
        assert_eq!(items.len(), 5);
        assert!(truncated);
    }

    #[test]
    fn requested_limit_is_clamped_to_the_hard_ceiling() {
        let fixture = TempDirectory::new("mention-search-clamp");
        let root = fixture.path();
        write_file(root, "src/only.rs");
        assert_eq!(
            walk_candidates(root, "only", usize::MAX)
                .expect("walk")
                .0
                .len(),
            1
        );
        assert_eq!(SEARCH_RESULT_CAP.clamp(1, SEARCH_RESULT_CAP), 50);
    }

    #[test]
    fn traversal_stops_at_the_depth_limit() {
        let fixture = TempDirectory::new("mention-search-depth");
        let root = fixture.path();
        let mut relative = String::new();
        for _ in 0..(SEARCH_DEPTH_LIMIT + 3) {
            relative.push_str("level/");
        }
        relative.push_str("buried.rs");
        write_file(root, &relative);
        write_file(root, "shallow.rs");
        let (items, truncated) = walk_candidates(root, "", 50).expect("walk");
        assert!(items.iter().all(|entry| entry.path != relative));
        assert!(truncated);
    }

    #[test]
    fn no_match_returns_an_empty_set_without_error() {
        let fixture = TempDirectory::new("mention-search-empty");
        let root = fixture.path();
        write_file(root, "src/utils.rs");
        let (items, truncated) = walk_candidates(root, "nothingmatchesthis", 50).expect("walk");
        assert!(items.is_empty());
        assert!(!truncated);
    }

    #[test]
    fn results_stay_inside_the_session_root() {
        let fixture = TempDirectory::new("mention-search-containment");
        let root = fixture.path().join("inside");
        fs::create_dir_all(&root).expect("root");
        write_file(&root, "src/kept.rs");
        write_file(fixture.path(), "outside/escaped.rs");
        let ordered = paths(&root, "", 50);
        assert_eq!(ordered, vec!["src/kept.rs".to_string()]);
    }
}
