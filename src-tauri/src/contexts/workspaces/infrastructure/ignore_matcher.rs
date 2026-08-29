//! Applying the ignore policy to a real workspace.
//!
//! The policy says what a recursive search is looking for; this reads the repository's own answer
//! to the same question. `.gitignore` and `.ignore` are already the file where a team records which
//! directories are generated, and a search that ignored them would offer build output from a
//! project that had explicitly written down that it is build output.
//!
//! One matcher implementation rather than a hand-rolled glob pass, because `.gitignore` syntax is
//! not what it looks like: anchoring depends on whether a pattern contains a slash, `**` and `*`
//! differ at directory boundaries, a trailing slash means directories only, and negation depends on
//! the order rules were read. The `ignore` crate already implements Git's rules and is already a
//! dependency of this workspace.
//!
//! Only the root's rule files are read. Nested `.gitignore` files apply to their own subtree, and
//! honouring them would mean an open/parse per directory entered — a per-directory cost paid on
//! every walk to change the answer for a minority of trees. The omission is recorded here rather
//! than discovered later.

use crate::contexts::workspaces::application::WorkspaceIgnorePolicy;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

/// The rule files read from a workspace root, in the order Git would read them.
const REPOSITORY_RULE_FILES: &[&str] = &[".gitignore", ".ignore"];

/// The policy plus whatever the repository itself says.
pub(crate) struct WorkspaceIgnoreMatcher {
    policy: WorkspaceIgnorePolicy,
    repository: Option<Gitignore>,
    identity: String,
}

impl WorkspaceIgnoreMatcher {
    /// Builds a matcher for one root.
    ///
    /// An unreadable or malformed rule file is not a failure. A search that refused to run because
    /// a `.gitignore` had a bad line would be unusable on exactly the repositories most likely to
    /// have one, so partial rules are kept and the rest are dropped.
    pub(crate) fn for_root(root: &Path, policy: WorkspaceIgnorePolicy) -> Self {
        if !policy.is_recursive_discovery() {
            return Self {
                policy,
                repository: None,
                identity: policy.identity(),
            };
        }
        let mut builder = GitignoreBuilder::new(root);
        let mut sources = 0usize;
        for name in REPOSITORY_RULE_FILES {
            let path = root.join(name);
            if !path.is_file() {
                continue;
            }
            // `add` returns the parse error rather than failing: the rules it did understand are
            // still the repository's own statement about its tree.
            let _ = builder.add(&path);
            sources += 1;
        }
        let repository = builder.build().ok().filter(|_| sources > 0);
        let identity = format!("{}:repo={sources}", policy.identity());
        Self {
            policy,
            repository,
            identity,
        }
    }

    /// Whether a recursive walk should skip this entry.
    ///
    /// `relative` is the workspace-relative path with forward slashes; `name` is the final
    /// component. Both are passed because the two rule sets ask different questions: the default
    /// exclusions are about a name anywhere in the tree, and repository rules are about a path.
    pub(crate) fn skips(&self, relative: &str, name: &str, is_directory: bool) -> bool {
        if !self.policy.is_recursive_discovery() {
            return false;
        }
        if self.policy.hides_dot_entries() && name.starts_with('.') {
            // Absolute, and deliberately not overridable. `.git` is the case that matters: a walk
            // that entered it would offer object files as search results, and no negation in a
            // `.gitignore` is a statement about wanting that.
            return true;
        }
        match self.repository_verdict(relative, is_directory) {
            // An explicit `!pattern` is a team saying they do want this tree searched, which is a
            // stronger statement than a default list written here.
            Some(true) => false,
            Some(false) => true,
            None => is_directory && self.policy.excludes_directory_name(name),
        }
    }

    /// `Some(true)` whitelisted, `Some(false)` ignored, `None` not mentioned.
    fn repository_verdict(&self, relative: &str, is_directory: bool) -> Option<bool> {
        let repository = self.repository.as_ref()?;
        let matched = repository.matched(Path::new(relative), is_directory);
        if matched.is_whitelist() {
            return Some(true);
        }
        if matched.is_ignore() {
            return Some(false);
        }
        None
    }

    /// A stable token naming which rules produced a result. No paths, no rule text.
    pub(crate) fn identity(&self) -> &str {
        &self.identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::fs;

    fn root_with(rules: &[(&str, &str)]) -> TempDirectory {
        let fixture = TempDirectory::new("ignore-matcher");
        for (name, contents) in rules {
            fs::write(fixture.path().join(name), contents).expect("rule file");
        }
        fixture
    }

    fn recursive(fixture: &TempDirectory) -> WorkspaceIgnoreMatcher {
        WorkspaceIgnoreMatcher::for_root(
            fixture.path(),
            WorkspaceIgnorePolicy::recursive_discovery(),
        )
    }

    #[test]
    fn the_default_trees_are_skipped_without_any_repository_rules() {
        let fixture = root_with(&[]);
        let matcher = recursive(&fixture);

        for name in ["node_modules", "target", "dist", "coverage", "vendor"] {
            assert!(matcher.skips(name, name, true), "{name}");
        }
        assert!(!matcher.skips("src", "src", true));
        assert!(!matcher.skips("src/main.rs", "main.rs", false));
    }

    #[test]
    fn a_dot_entry_is_hidden_and_a_negation_cannot_bring_it_back() {
        let fixture = root_with(&[(".gitignore", "!.git\n")]);
        let matcher = recursive(&fixture);

        // A walk that entered `.git` would offer object files as search results, and no `!` in a
        // rule file is a statement about wanting that.
        assert!(matcher.skips(".git", ".git", true));
    }

    #[test]
    fn a_repository_rule_skips_a_tree_the_defaults_never_heard_of() {
        let fixture = root_with(&[(".gitignore", "generated/\n*.snap\n")]);
        let matcher = recursive(&fixture);

        assert!(matcher.skips("generated", "generated", true));
        assert!(matcher.skips("src/button.snap", "button.snap", false));
        assert!(!matcher.skips("src/button.tsx", "button.tsx", false));
    }

    #[test]
    fn a_repository_negation_re_includes_a_default_excluded_tree() {
        let fixture = root_with(&[(".gitignore", "!vendor/\n")]);
        let matcher = recursive(&fixture);

        // An explicit `!` is a team saying they do want this tree searched, which is a stronger
        // statement than a default list written in this repository.
        assert!(!matcher.skips("vendor", "vendor", true));
        assert!(matcher.skips("node_modules", "node_modules", true));
    }

    #[test]
    fn a_dot_ignore_file_is_read_alongside_gitignore() {
        let fixture = root_with(&[(".gitignore", "from-git/\n"), (".ignore", "from-ignore/\n")]);
        let matcher = recursive(&fixture);

        assert!(matcher.skips("from-git", "from-git", true));
        assert!(matcher.skips("from-ignore", "from-ignore", true));
    }

    #[test]
    fn a_nested_rule_applies_where_it_was_written() {
        let fixture = root_with(&[(".gitignore", "src/generated/\n")]);
        let matcher = recursive(&fixture);

        assert!(matcher.skips("src/generated", "generated", true));
        // Anchored by the slash in the pattern, so a directory of the same name elsewhere stays.
        assert!(!matcher.skips("docs/generated", "generated", true));
    }

    #[test]
    fn a_malformed_rule_file_leaves_the_walk_running() {
        let fixture = root_with(&[(".gitignore", "generated/\n[unclosed\n")]);
        let matcher = recursive(&fixture);

        // A search that refused to run because a `.gitignore` had a bad line would be unusable on
        // exactly the repositories most likely to have one.
        assert!(matcher.skips("generated", "generated", true));
        assert!(!matcher.skips("src", "src", true));
    }

    #[test]
    fn direct_navigation_skips_nothing_even_with_rules_present() {
        let fixture = root_with(&[(".gitignore", "generated/\n")]);
        let matcher = WorkspaceIgnoreMatcher::for_root(
            fixture.path(),
            WorkspaceIgnorePolicy::direct_navigation(),
        );

        // Ignore is not authorization. A reader who navigated here has said what they want, and the
        // root, type and size rules that actually protect something are unchanged.
        assert!(!matcher.skips("node_modules", "node_modules", true));
        assert!(!matcher.skips("generated", "generated", true));
        assert!(!matcher.skips(".git", ".git", true));
    }

    #[test]
    fn the_identity_records_how_many_rule_files_were_read() {
        let none = root_with(&[]);
        let one = root_with(&[(".gitignore", "x\n")]);

        // A cursor issued while a repository had no rule file must not be resumed after one
        // appeared: the page it named was computed under different rules.
        assert_ne!(recursive(&none).identity(), recursive(&one).identity());
        assert!(!recursive(&one).identity().contains('/'));
    }
}
