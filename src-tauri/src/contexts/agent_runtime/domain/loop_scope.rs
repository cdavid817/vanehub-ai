//! Versioned Loop mutation scope: the literal worktree-relative upper bound a run may change.
//!
//! The scope is a pure value. It knows nothing about the filesystem it will be applied to; the
//! infrastructure adapter resolves real resources and asks this module to classify the component
//! path it found. Keeping the two apart is what lets the same classifier serve saving, readiness,
//! authoritative start, mediated tool delivery and artifact evidence scans without drifting.

use sha2::{Digest, Sha256};
use std::fmt;

/// The only scope syntax this build understands. A definition stored with another version is
/// treated as legacy-unverified and must be edited before it can start.
pub(crate) const LOOP_SCOPE_SCHEMA_VERSION: u32 = 1;
pub(crate) const MAX_SCOPE_ENTRIES: usize = 256;
pub(crate) const MAX_SCOPE_ENTRY_BYTES: usize = 1024;
pub(crate) const MAX_SCOPE_TOTAL_BYTES: usize = 32 * 1024;

/// Which enforcement the user asked for. `PreventiveRequired` is the default for new definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LoopRequestedMode {
    PreventiveRequired,
    ArtifactAudited,
}

impl LoopRequestedMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PreventiveRequired => "preventive-required",
            Self::ArtifactAudited => "artifact-audited",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "preventive-required" => Some(Self::PreventiveRequired),
            "artifact-audited" => Some(Self::ArtifactAudited),
            _ => None,
        }
    }
}

/// How much of a role's or surface's reachable mutation channels are constrained before effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum LoopCoverage {
    CompleteEnforcement,
    MediatedToolsOnly,
    ArtifactValidationOnly,
    Unsupported,
    Unknown,
}

impl LoopCoverage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CompleteEnforcement => "complete-enforcement",
            Self::MediatedToolsOnly => "mediated-tools-only",
            Self::ArtifactValidationOnly => "artifact-validation-only",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "complete-enforcement" => Some(Self::CompleteEnforcement),
            "mediated-tools-only" => Some(Self::MediatedToolsOnly),
            "artifact-validation-only" => Some(Self::ArtifactValidationOnly),
            "unsupported" => Some(Self::Unsupported),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Whether this coverage can satisfy the requested mode on its own.
    pub(crate) fn satisfies(self, mode: LoopRequestedMode) -> bool {
        match mode {
            LoopRequestedMode::PreventiveRequired => self == Self::CompleteEnforcement,
            LoopRequestedMode::ArtifactAudited => matches!(
                self,
                Self::CompleteEnforcement | Self::MediatedToolsOnly | Self::ArtifactValidationOnly
            ),
        }
    }
}

/// A side-effect channel a Loop-owned execution could reach. Every host-received request names
/// the channel it uses so admission can refuse the ones the role's coverage does not cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LoopSideEffectChannel {
    MediatedFile,
    Shell,
    Terminal,
    Mcp,
    SkillTool,
    Delegation,
    Notebook,
    CliInternal,
}

impl LoopSideEffectChannel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MediatedFile => "mediated-file",
            Self::Shell => "shell",
            Self::Terminal => "terminal",
            Self::Mcp => "mcp",
            Self::SkillTool => "skill-tool",
            Self::Delegation => "delegation",
            Self::Notebook => "notebook",
            Self::CliInternal => "cli-internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoopScopeError {
    EmptyEntry,
    ParentTraversal(String),
    AbsolutePath(String),
    DrivePath(String),
    UncOrDevicePath(String),
    AlternateDataStream(String),
    ControlCharacters(String),
    GlobSyntax(String),
    ReservedResource(String),
    EntryTooLong(String),
    TooManyEntries(usize),
    TotalTooLarge(usize),
    EmptyAllowedScope,
    AllowedCoveredByProtection(String),
}

impl LoopScopeError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::EmptyEntry => "scope-empty-entry",
            Self::ParentTraversal(_) => "scope-parent-traversal",
            Self::AbsolutePath(_) => "scope-absolute-path",
            Self::DrivePath(_) => "scope-drive-path",
            Self::UncOrDevicePath(_) => "scope-unc-or-device-path",
            Self::AlternateDataStream(_) => "scope-alternate-data-stream",
            Self::ControlCharacters(_) => "scope-control-characters",
            Self::GlobSyntax(_) => "scope-glob-syntax",
            Self::ReservedResource(_) => "scope-reserved-resource",
            Self::EntryTooLong(_) => "scope-entry-too-long",
            Self::TooManyEntries(_) => "scope-too-many-entries",
            Self::TotalTooLarge(_) => "scope-total-too-large",
            Self::EmptyAllowedScope => "scope-empty-allowed",
            Self::AllowedCoveredByProtection(_) => "scope-allowed-covered",
        }
    }
}

impl fmt::Display for LoopScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEntry => write!(f, "scope entries must not be empty"),
            Self::ParentTraversal(entry) => {
                write!(f, "scope entry \"{entry}\" uses parent traversal")
            }
            Self::AbsolutePath(entry) => write!(f, "scope entry \"{entry}\" is absolute"),
            Self::DrivePath(entry) => write!(f, "scope entry \"{entry}\" names a drive"),
            Self::UncOrDevicePath(entry) => {
                write!(f, "scope entry \"{entry}\" is a UNC or device path")
            }
            Self::AlternateDataStream(entry) => {
                write!(f, "scope entry \"{entry}\" names an alternate data stream")
            }
            Self::ControlCharacters(entry) => {
                write!(f, "scope entry \"{entry}\" contains control characters")
            }
            Self::GlobSyntax(entry) => write!(f, "scope entry \"{entry}\" uses glob syntax"),
            Self::ReservedResource(entry) => {
                write!(
                    f,
                    "scope entry \"{entry}\" names a reserved control resource"
                )
            }
            Self::EntryTooLong(entry) => write!(f, "scope entry \"{entry}\" is too long"),
            Self::TooManyEntries(count) => {
                write!(
                    f,
                    "scope has {count} entries; at most {MAX_SCOPE_ENTRIES} are allowed"
                )
            }
            Self::TotalTooLarge(bytes) => write!(
                f,
                "scope configuration is {bytes} bytes; at most {MAX_SCOPE_TOTAL_BYTES} are allowed"
            ),
            Self::EmptyAllowedScope => write!(f, "at least one allowed path is required"),
            Self::AllowedCoveredByProtection(entry) => write!(
                f,
                "allowed path \"{entry}\" is entirely covered by a protected path"
            ),
        }
    }
}

/// A normalized worktree-relative path. The root is the empty component list, written `.`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LoopScopePath {
    components: Vec<String>,
}

impl LoopScopePath {
    pub(crate) fn from_components(components: Vec<String>) -> Self {
        Self { components }
    }

    /// Parses one configuration entry. Windows-style `\` separators are accepted as separators;
    /// nothing else is reinterpreted.
    pub(crate) fn parse(raw: &str) -> Result<Self, LoopScopeError> {
        let entry = raw.trim();
        if entry.is_empty() {
            return Err(LoopScopeError::EmptyEntry);
        }
        if entry.len() > MAX_SCOPE_ENTRY_BYTES {
            return Err(LoopScopeError::EntryTooLong(bounded(entry)));
        }
        if entry.chars().any(char::is_control) {
            return Err(LoopScopeError::ControlCharacters(bounded(entry)));
        }
        if entry.starts_with("\\\\") || entry.starts_with("//") {
            return Err(LoopScopeError::UncOrDevicePath(bounded(entry)));
        }
        if entry.starts_with(['/', '\\']) {
            return Err(LoopScopeError::AbsolutePath(bounded(entry)));
        }
        if entry.as_bytes().get(1) == Some(&b':') && entry.as_bytes()[0].is_ascii_alphabetic() {
            return Err(LoopScopeError::DrivePath(bounded(entry)));
        }
        if entry.contains(':') {
            return Err(LoopScopeError::AlternateDataStream(bounded(entry)));
        }
        if entry.contains(['*', '?', '[', ']', '{', '}']) {
            return Err(LoopScopeError::GlobSyntax(bounded(entry)));
        }
        let mut components = Vec::new();
        for component in entry.split(['/', '\\']) {
            match component {
                "" | "." => continue,
                ".." => return Err(LoopScopeError::ParentTraversal(bounded(entry))),
                other => components.push(other.to_string()),
            }
        }
        Ok(Self { components })
    }

    pub(crate) fn is_root(&self) -> bool {
        self.components.is_empty()
    }

    pub(crate) fn components(&self) -> &[String] {
        &self.components
    }

    /// Git control metadata is a host-owned resource at any depth: nested repositories and
    /// worktree link files are just as much off limits as the top-level one.
    pub(crate) fn is_reserved(&self) -> bool {
        self.components
            .iter()
            .any(|component| component.eq_ignore_ascii_case(".git"))
    }

    pub(crate) fn display(&self) -> String {
        if self.is_root() {
            ".".to_string()
        } else {
            self.components.join("/")
        }
    }

    /// Whether `self` is `ancestor` or one of its descendants under the volume's case rule.
    fn is_within(&self, ancestor: &Self, case: CaseRule) -> Containment {
        if ancestor.components.len() > self.components.len() {
            return Containment::Outside;
        }
        let mut ambiguous = false;
        for (mine, theirs) in self.components.iter().zip(&ancestor.components) {
            match component_relation(mine, theirs, case) {
                ComponentRelation::Equal => {}
                ComponentRelation::Different => return Containment::Outside,
                ComponentRelation::Ambiguous => ambiguous = true,
            }
        }
        if ambiguous {
            Containment::Ambiguous
        } else {
            Containment::Inside
        }
    }
}

/// How the target volume compares names. Decided by the infrastructure adapter from the real
/// worktree, never assumed per platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaseRule {
    Sensitive,
    /// ASCII letters fold; anything beyond ASCII cannot be folded without knowing the volume's
    /// Unicode tables, so differing non-ASCII names are reported as ambiguous.
    InsensitiveAscii,
}

impl CaseRule {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Sensitive => "sensitive",
            Self::InsensitiveAscii => "insensitive-ascii",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentRelation {
    Equal,
    Different,
    Ambiguous,
}

fn component_relation(left: &str, right: &str, case: CaseRule) -> ComponentRelation {
    if left == right {
        return ComponentRelation::Equal;
    }
    match case {
        CaseRule::Sensitive => ComponentRelation::Different,
        CaseRule::InsensitiveAscii => {
            if left.eq_ignore_ascii_case(right) {
                ComponentRelation::Equal
            } else if left.is_ascii() && right.is_ascii() {
                ComponentRelation::Different
            } else {
                ComponentRelation::Ambiguous
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Containment {
    Inside,
    Outside,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeClassification {
    Reserved,
    Protected,
    Allowed,
    Outside,
}

impl ScopeClassification {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Protected => "protected",
            Self::Allowed => "allowed",
            Self::Outside => "outside",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScopeRejection {
    pub(crate) code: &'static str,
    pub(crate) path: String,
}

impl ScopeRejection {
    pub(crate) fn message(&self) -> String {
        format!("{}: {}", self.code, self.path)
    }
}

/// The normalized, validated scope of one definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopScopeConfig {
    allowed: Vec<LoopScopePath>,
    protected: Vec<LoopScopePath>,
}

impl LoopScopeConfig {
    pub(crate) fn parse(allowed: &[String], protected: &[String]) -> Result<Self, LoopScopeError> {
        let total = allowed
            .iter()
            .chain(protected)
            .map(String::len)
            .sum::<usize>();
        if total > MAX_SCOPE_TOTAL_BYTES {
            return Err(LoopScopeError::TotalTooLarge(total));
        }
        if allowed.len() + protected.len() > MAX_SCOPE_ENTRIES {
            return Err(LoopScopeError::TooManyEntries(
                allowed.len() + protected.len(),
            ));
        }
        let mut allowed = allowed
            .iter()
            .map(|entry| {
                let path = LoopScopePath::parse(entry)?;
                // Git metadata and other control resources can never be opened for mutation,
                // not even by naming them explicitly. Listing them as protected is harmless.
                if path.is_reserved() {
                    return Err(LoopScopeError::ReservedResource(entry.trim().to_string()));
                }
                Ok(path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut protected = protected
            .iter()
            .map(|entry| LoopScopePath::parse(entry))
            .collect::<Result<Vec<_>, _>>()?;
        allowed.sort();
        allowed.dedup();
        protected.sort();
        protected.dedup();
        if allowed.is_empty() {
            return Err(LoopScopeError::EmptyAllowedScope);
        }
        // An allowed entry that sits inside a protected subtree has no usable scope. The stricter
        // rule wins, so the configuration is rejected rather than silently reduced.
        for entry in &allowed {
            if protected
                .iter()
                .any(|shield| entry.is_within(shield, CaseRule::Sensitive) == Containment::Inside)
            {
                return Err(LoopScopeError::AllowedCoveredByProtection(entry.display()));
            }
        }
        Ok(Self { allowed, protected })
    }

    #[cfg(test)]
    pub(crate) fn allowed(&self) -> &[LoopScopePath] {
        &self.allowed
    }

    pub(crate) fn allowed_display(&self) -> Vec<String> {
        self.allowed.iter().map(LoopScopePath::display).collect()
    }

    pub(crate) fn protected_display(&self) -> Vec<String> {
        self.protected.iter().map(LoopScopePath::display).collect()
    }

    /// Classifies one target. Reserved beats protected beats allowed; an ambiguous comparison is
    /// resolved towards the stricter outcome on every branch.
    pub(crate) fn classify(&self, target: &LoopScopePath, case: CaseRule) -> ScopeClassification {
        if target.is_reserved() {
            return ScopeClassification::Reserved;
        }
        if self.protected.iter().any(|shield| {
            matches!(
                target.is_within(shield, case),
                Containment::Inside | Containment::Ambiguous
            )
        }) {
            return ScopeClassification::Protected;
        }
        if self
            .allowed
            .iter()
            .any(|entry| target.is_within(entry, case) == Containment::Inside)
        {
            ScopeClassification::Allowed
        } else {
            ScopeClassification::Outside
        }
    }

    /// Admission for one mutation target. Every affected target of a compound operation must be
    /// admitted individually before any of them changes.
    pub(crate) fn admit_mutation(
        &self,
        target: &LoopScopePath,
        case: CaseRule,
    ) -> Result<(), ScopeRejection> {
        match self.classify(target, case) {
            ScopeClassification::Allowed => Ok(()),
            ScopeClassification::Reserved => Err(ScopeRejection {
                code: "scope-reserved-resource",
                path: target.display(),
            }),
            ScopeClassification::Protected => Err(ScopeRejection {
                code: "scope-protected-path",
                path: target.display(),
            }),
            ScopeClassification::Outside => Err(ScopeRejection {
                code: "scope-outside-allowed",
                path: target.display(),
            }),
        }
    }

    /// A stable digest of the complete scope and mode. Prompt summaries may truncate; this never
    /// does, so an executor comparing digests is comparing the full configuration.
    pub(crate) fn digest(&self, mode: LoopRequestedMode) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"loop-scope-v1\0");
        hasher.update(mode.as_str().as_bytes());
        hasher.update(b"\0allowed\0");
        for entry in &self.allowed {
            hasher.update(entry.display().as_bytes());
            hasher.update(b"\0");
        }
        hasher.update(b"protected\0");
        for entry in &self.protected {
            hasher.update(entry.display().as_bytes());
            hasher.update(b"\0");
        }
        let digest = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("sha256:{digest}")
    }
}

fn bounded(entry: &str) -> String {
    entry.chars().take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(allowed: &[&str], protected: &[&str]) -> LoopScopeConfig {
        LoopScopeConfig::parse(
            &allowed
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            &protected
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("valid scope")
    }

    fn path(value: &str) -> LoopScopePath {
        LoopScopePath::parse(value).expect("valid path")
    }

    #[test]
    fn components_match_whole_names_and_protected_descendants_win() {
        let scope = config(&["src"], &["src/generated"]);
        assert_eq!(
            scope.classify(&path("src/app.ts"), CaseRule::Sensitive),
            ScopeClassification::Allowed
        );
        assert_eq!(
            scope.classify(&path("src-old/app.ts"), CaseRule::Sensitive),
            ScopeClassification::Outside
        );
        assert_eq!(
            scope.classify(&path("src/generated/a.ts"), CaseRule::Sensitive),
            ScopeClassification::Protected
        );
        assert_eq!(
            scope.classify(&path("src"), CaseRule::Sensitive),
            ScopeClassification::Allowed
        );
        assert!(scope
            .admit_mutation(&path("src/generated"), CaseRule::Sensitive)
            .is_err());
    }

    #[test]
    fn root_selects_the_whole_worktree_minus_protection_and_reserved_resources() {
        let scope = config(&["."], &["docs"]);
        assert_eq!(
            scope.classify(&path("anything/here.txt"), CaseRule::Sensitive),
            ScopeClassification::Allowed
        );
        assert_eq!(
            scope.classify(&path("docs/index.md"), CaseRule::Sensitive),
            ScopeClassification::Protected
        );
        assert_eq!(
            scope.classify(&path(".git/HEAD"), CaseRule::Sensitive),
            ScopeClassification::Reserved
        );
        assert_eq!(
            scope.classify(&path("nested/.git/config"), CaseRule::Sensitive),
            ScopeClassification::Reserved
        );
        assert_eq!(scope.allowed_display(), vec![".".to_string()]);
    }

    #[test]
    fn rejects_escaping_ambiguous_and_reserved_configuration() {
        let cases: &[(&str, &str)] = &[
            ("../src", "scope-parent-traversal"),
            ("src/../lib", "scope-parent-traversal"),
            ("/etc", "scope-absolute-path"),
            ("\\Windows", "scope-absolute-path"),
            ("C:\\repo", "scope-drive-path"),
            ("\\\\server\\share", "scope-unc-or-device-path"),
            ("//server/share", "scope-unc-or-device-path"),
            ("file.txt:stream", "scope-alternate-data-stream"),
            ("src/*.ts", "scope-glob-syntax"),
            ("src/{a,b}", "scope-glob-syntax"),
            ("src\u{0}bad", "scope-control-characters"),
            ("   ", "scope-empty-entry"),
        ];
        for (entry, code) in cases {
            let error = LoopScopePath::parse(entry).expect_err(entry);
            assert_eq!(error.code(), *code, "{entry}");
        }
        for entry in [".git", ".GIT/objects", "nested/.git"] {
            assert_eq!(
                LoopScopeConfig::parse(&[entry.to_string()], &[])
                    .expect_err(entry)
                    .code(),
                "scope-reserved-resource"
            );
        }
        // Protecting a reserved resource is redundant but never wrong.
        assert!(LoopScopeConfig::parse(&["src".to_string()], &[".git".to_string()]).is_ok());
        assert_eq!(
            LoopScopeConfig::parse(&[], &[]).expect_err("empty").code(),
            "scope-empty-allowed"
        );
        assert_eq!(
            LoopScopeConfig::parse(&["src".to_string()], &["src".to_string()])
                .expect_err("covered")
                .code(),
            "scope-allowed-covered"
        );
        assert_eq!(
            LoopScopeConfig::parse(&["src/app".to_string()], &["src".to_string()])
                .expect_err("covered descendant")
                .code(),
            "scope-allowed-covered"
        );
    }

    #[test]
    fn normalizes_separators_duplicates_and_ordering_without_touching_names() {
        let scope = config(&["src//lib/", ".\\src\\lib", "b", "a"], &[]);
        assert_eq!(scope.allowed_display(), vec!["a", "b", "src/lib"]);
        assert_eq!(
            path("dir with space/file name.txt").display(),
            "dir with space/file name.txt"
        );
        assert_eq!(path("./").display(), ".");
    }

    #[test]
    fn capacity_overflow_is_rejected_not_truncated() {
        let many = (0..MAX_SCOPE_ENTRIES + 1)
            .map(|index| format!("dir-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            LoopScopeConfig::parse(&many, &[])
                .expect_err("too many")
                .code(),
            "scope-too-many-entries"
        );
        let long = "a".repeat(MAX_SCOPE_ENTRY_BYTES + 1);
        assert_eq!(
            LoopScopePath::parse(&long).expect_err("too long").code(),
            "scope-entry-too-long"
        );
        let wide = vec![
            "x".repeat(MAX_SCOPE_ENTRY_BYTES);
            MAX_SCOPE_TOTAL_BYTES / MAX_SCOPE_ENTRY_BYTES + 1
        ];
        assert_eq!(
            LoopScopeConfig::parse(&wide, &[])
                .expect_err("too large")
                .code(),
            "scope-total-too-large"
        );
        let full = (0..MAX_SCOPE_ENTRIES)
            .map(|index| format!("dir-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            LoopScopeConfig::parse(&full, &[])
                .expect("at capacity")
                .allowed()
                .len(),
            MAX_SCOPE_ENTRIES
        );
    }

    #[test]
    fn case_rule_comes_from_the_volume_and_non_ascii_folds_are_never_guessed() {
        let scope = config(&["src"], &["src/Generated"]);
        assert_eq!(
            scope.classify(&path("SRC/app.ts"), CaseRule::Sensitive),
            ScopeClassification::Outside
        );
        assert_eq!(
            scope.classify(&path("SRC/app.ts"), CaseRule::InsensitiveAscii),
            ScopeClassification::Allowed
        );
        assert_eq!(
            scope.classify(&path("src/generated/x"), CaseRule::InsensitiveAscii),
            ScopeClassification::Protected
        );
        let unicode = config(&["docs"], &["docs/Ünterlagen"]);
        // A non-ASCII name that differs only by case cannot be folded safely: it is treated as
        // protected (stricter) rather than as an unrelated sibling.
        assert_eq!(
            unicode.classify(&path("docs/ünterlagen/a"), CaseRule::InsensitiveAscii),
            ScopeClassification::Protected
        );
        let allowed_unicode = config(&["Ünterlagen"], &[]);
        assert_eq!(
            allowed_unicode.classify(&path("ünterlagen/a"), CaseRule::InsensitiveAscii),
            ScopeClassification::Outside
        );
    }

    #[test]
    fn digest_covers_every_entry_and_the_mode() {
        let a = config(&["src"], &["src/generated"]);
        let b = config(&["src"], &[]);
        assert_ne!(
            a.digest(LoopRequestedMode::PreventiveRequired),
            b.digest(LoopRequestedMode::PreventiveRequired)
        );
        assert_ne!(
            a.digest(LoopRequestedMode::PreventiveRequired),
            a.digest(LoopRequestedMode::ArtifactAudited)
        );
        assert_eq!(
            a.digest(LoopRequestedMode::PreventiveRequired),
            config(&["src/"], &["src/generated/"]).digest(LoopRequestedMode::PreventiveRequired)
        );
        assert!(LoopCoverage::CompleteEnforcement.satisfies(LoopRequestedMode::PreventiveRequired));
        assert!(!LoopCoverage::MediatedToolsOnly.satisfies(LoopRequestedMode::PreventiveRequired));
        assert!(LoopCoverage::ArtifactValidationOnly.satisfies(LoopRequestedMode::ArtifactAudited));
        assert!(!LoopCoverage::Unknown.satisfies(LoopRequestedMode::ArtifactAudited));
    }
}
