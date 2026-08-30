//! Which parts of a workspace a recursive search is looking for, and which it is not.
//!
//! Three walks had their own copy of the same exclusion list and a fourth had none at all: Quick
//! Open and content search skipped `node_modules` and its relatives, mention candidates skipped the
//! same names from a second constant, and document discovery descended into every one of them. A
//! reader could therefore find a file by name and not by content, or find a vendored README in the
//! Documents tab that no other surface would ever offer. One policy, consulted by all four.
//!
//! It is a discovery rule, not an authorization rule, and the distinction is the whole reason the
//! mode exists. Recursive discovery is a guess about what somebody meant when they typed a few
//! characters, and guessing that they did not mean the contents of a dependency tree is usually
//! right. Direct navigation is not a guess: a reader who typed a path or clicked a folder has said
//! exactly what they want, and hiding it because a `.gitignore` mentions it would be answering a
//! different question. Root confinement, size limits and type checks are unchanged by either mode —
//! they are the rules that actually protect something.

/// Whether a rule set is being applied to a search or to a request for one named thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceIgnoreMode {
    /// A walk looking for entries nobody named. Defaults and repository ignore rules apply.
    RecursiveDiscovery,
    /// One path a reader asked for. Nothing here hides it.
    DirectNavigation,
}

/// The version of the default rule set.
///
/// Part of the policy identity, so a cursor issued under one set of defaults can be told apart from
/// one issued under another. Bumped when `DEFAULT_EXCLUDED_DIRECTORIES` changes in a way that moves
/// entries into or out of a listing.
const POLICY_VERSION: u32 = 1;

/// Directory names a recursive search does not descend into.
///
/// The union of what Quick Open, content search and mention candidates each excluded, plus the
/// generated-output directories from this change's design that none of them had: `.next`, `.nuxt`
/// and `.pytest_cache`. `.git` is covered by the dot rule rather than listed here.
///
/// `bin` is deliberately absent: Cargo treats `src/bin` as real source, and a reader looking for a
/// binary's entry point would find nothing.
const DEFAULT_EXCLUDED_DIRECTORIES: &[&str] = &[
    "__pycache__",
    "bower_components",
    "build",
    "coverage",
    "deriveddata",
    "dist",
    "jspm_packages",
    "node_modules",
    "obj",
    "out",
    "pods",
    "site-packages",
    "target",
    "venv",
    "vendor",
    ".next",
    ".nuxt",
    ".pytest_cache",
];

/// What a walk is allowed to look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkspaceIgnorePolicy {
    mode: WorkspaceIgnoreMode,
}

impl WorkspaceIgnorePolicy {
    pub(crate) fn recursive_discovery() -> Self {
        Self {
            mode: WorkspaceIgnoreMode::RecursiveDiscovery,
        }
    }

    pub(crate) fn direct_navigation() -> Self {
        Self {
            mode: WorkspaceIgnoreMode::DirectNavigation,
        }
    }

    pub(crate) fn mode(self) -> WorkspaceIgnoreMode {
        self.mode
    }

    pub(crate) fn is_recursive_discovery(self) -> bool {
        self.mode == WorkspaceIgnoreMode::RecursiveDiscovery
    }

    pub(crate) fn default_excluded_directories() -> &'static [&'static str] {
        DEFAULT_EXCLUDED_DIRECTORIES
    }

    /// Whether a directory name is in the default exclusion set.
    ///
    /// Case-insensitive, because `Pods` and `DerivedData` are the spellings those tools actually
    /// produce and a case-sensitive list would miss both on the platforms they come from.
    pub(crate) fn excludes_directory_name(self, name: &str) -> bool {
        if !self.is_recursive_discovery() {
            return false;
        }
        let lowered = name.to_ascii_lowercase();
        DEFAULT_EXCLUDED_DIRECTORIES
            .iter()
            .any(|excluded| *excluded == lowered)
    }

    /// Whether a dot-prefixed entry is hidden from this walk.
    ///
    /// Absolute in recursive discovery, and not overridable by a repository rule. `.git` is the
    /// case that matters: a walk that entered it would offer object files as search results, and
    /// no negation in a `.gitignore` is a statement about wanting that.
    pub(crate) fn hides_dot_entries(self) -> bool {
        self.is_recursive_discovery()
    }

    /// A stable token naming which rules produced a result.
    ///
    /// Carried on a cursor so a page issued under one policy is not resumed under another, and
    /// useful in a diagnostic. It names the version and the mode and nothing else: the rules
    /// themselves can mention paths a user has a reason to keep private.
    pub(crate) fn identity(self) -> String {
        let mode = match self.mode {
            WorkspaceIgnoreMode::RecursiveDiscovery => "recursive",
            WorkspaceIgnoreMode::DirectNavigation => "direct",
        };
        format!("v{POLICY_VERSION}:{mode}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_discovery_skips_the_generated_and_dependency_trees() {
        let policy = WorkspaceIgnorePolicy::recursive_discovery();

        for name in [
            "node_modules",
            "target",
            "dist",
            "build",
            "coverage",
            ".next",
            ".nuxt",
            "vendor",
            "__pycache__",
            ".pytest_cache",
        ] {
            assert!(policy.excludes_directory_name(name), "{name}");
        }
    }

    #[test]
    fn direct_navigation_hides_nothing_at_all() {
        let policy = WorkspaceIgnorePolicy::direct_navigation();

        // Ignore policy is a discovery rule, not an access-control decision. A reader who typed a
        // path has said what they want, and refusing it because a walk would have skipped it would
        // answer a different question.
        assert!(!policy.excludes_directory_name("node_modules"));
        assert!(!policy.hides_dot_entries());
    }

    #[test]
    fn a_cargo_style_src_bin_stays_searchable() {
        // Cargo treats `src/bin` as real source, and a reader looking for a binary's entry point
        // would otherwise find nothing.
        assert!(!WorkspaceIgnorePolicy::recursive_discovery().excludes_directory_name("bin"));
    }

    #[test]
    fn exclusion_matching_ignores_case() {
        let policy = WorkspaceIgnorePolicy::recursive_discovery();

        // `Pods` and `DerivedData` are the spellings those tools produce.
        assert!(policy.excludes_directory_name("Pods"));
        assert!(policy.excludes_directory_name("DerivedData"));
        assert!(policy.excludes_directory_name("NODE_MODULES"));
    }

    #[test]
    fn the_identity_names_the_rules_without_naming_a_path() {
        let recursive = WorkspaceIgnorePolicy::recursive_discovery().identity();
        let direct = WorkspaceIgnorePolicy::direct_navigation().identity();

        assert_ne!(recursive, direct);
        // A cursor carries this, and a cursor travels to a frontend. Rules can mention paths a user
        // has a reason to keep private; a version and a mode cannot.
        assert!(!recursive.contains('/'));
        assert!(!recursive.contains('\\'));
        assert!(recursive.starts_with("v1:"));
    }

    #[test]
    fn the_default_set_is_sorted_and_free_of_duplicates() {
        // Not cosmetic: three lists drifted apart before this one existed, and the cheapest way to
        // see a rule arrive twice under two spellings is to require an order.
        let names = WorkspaceIgnorePolicy::default_excluded_directories();
        let mut deduplicated = names.to_vec();
        deduplicated.sort_unstable();
        deduplicated.dedup();
        assert_eq!(deduplicated.len(), names.len());
        for name in names {
            assert_eq!(*name, name.to_ascii_lowercase(), "{name}");
        }
    }
}
