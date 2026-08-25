//! The set of languages this build can serve.
//!
//! Everything a language needs is declared in one entry: how to find its server, how to start it,
//! how to recognize its project root, which files belong to it, and what minimal project the
//! isolated server test should build. Before this table the same knowledge was spread across a
//! two-variant enum, a discovery preset, a root-marker function, an extension match, and a
//! server-test match, so adding a language meant finding all five.
//!
//! The table is compile-time on purpose. Each entry needs a fixture project and root-detection
//! rules that only code can supply, so a user-declared language would be a row the runtime cannot
//! actually serve.

use super::language_id::LspLanguageId;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostPlatform {
    Windows,
    Macos,
    Linux,
}

impl HostPlatform {
    pub(crate) const fn current() -> Self {
        #[cfg(windows)]
        {
            Self::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            Self::Linux
        }
    }
}

const EVERY_PLATFORM: &[HostPlatform] = &[
    HostPlatform::Windows,
    HostPlatform::Macos,
    HostPlatform::Linux,
];

/// One file of a language's isolated server-test project, as a relative path and its contents.
pub(crate) type FixtureFile = (&'static str, &'static str);

/// One admitted source-file extension and the LSP `languageId` it maps to. The two differ: `.tsx`
/// belongs to the TypeScript/JavaScript language but must be announced as `typescriptreact`.
pub(crate) type ExtensionMapping = (&'static str, &'static str);

pub(crate) struct LanguageDefinition {
    pub(crate) id: &'static str,
    pub(crate) server_id: &'static str,
    /// Candidate executable names in preference order. A language may ship under more than one
    /// name, and the first that resolves wins.
    pub(crate) executables: &'static [&'static str],
    pub(crate) default_startup_arguments: &'static [&'static str],
    pub(crate) root_markers: &'static [&'static str],
    pub(crate) extensions: &'static [ExtensionMapping],
    pub(crate) fixture_files: &'static [FixtureFile],
    pub(crate) platforms: &'static [HostPlatform],
}

/// Identity is the language id alone. Two references to the same entry must compare equal without
/// walking every declared marker and extension, and the registry-completeness test already proves
/// no two entries share an id.
impl PartialEq for LanguageDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LanguageDefinition {}

impl Ord for LanguageDefinition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(other.id)
    }
}

impl PartialOrd for LanguageDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for LanguageDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// The derived form would print every declared marker, extension, and fixture file, which buries
/// the one field that identifies the entry in log lines and assertion output.
impl fmt::Debug for LanguageDefinition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "LanguageDefinition({})", self.id)
    }
}

impl LanguageDefinition {
    pub(crate) fn language_id(&self) -> LspLanguageId {
        LspLanguageId::trusted(self.id)
    }

    pub(crate) fn supports_host(&self) -> bool {
        self.platforms.contains(&HostPlatform::current())
    }

    pub(crate) fn language_id_for_extension(&self, extension: &str) -> Option<&'static str> {
        self.extensions
            .iter()
            .find(|(candidate, _)| *candidate == extension)
            .map(|(_, language_id)| *language_id)
    }
}

pub(crate) const LANGUAGE_DEFINITIONS: &[LanguageDefinition] = &[
    LanguageDefinition {
        id: "rust",
        server_id: "rust_analyzer",
        executables: &["rust-analyzer"],
        default_startup_arguments: &[],
        root_markers: &["Cargo.toml"],
        extensions: &[("rs", "rust")],
        fixture_files: &[
            (
                "Cargo.toml",
                "[package]\nname='lsp_test'\nversion='0.1.0'\n",
            ),
            ("src/lib.rs", "pub fn fixture() {}\n"),
        ],
        platforms: EVERY_PLATFORM,
    },
    LanguageDefinition {
        id: "typescript_javascript",
        server_id: "typescript_language_server",
        executables: &["typescript-language-server"],
        default_startup_arguments: &["--stdio"],
        root_markers: &["tsconfig.json", "jsconfig.json", "package.json"],
        extensions: &[
            ("ts", "typescript"),
            ("tsx", "typescriptreact"),
            ("js", "javascript"),
            ("mjs", "javascript"),
            ("cjs", "javascript"),
            ("jsx", "javascriptreact"),
        ],
        fixture_files: &[
            ("package.json", "{\"private\":true}"),
            ("tsconfig.json", "{\"compilerOptions\":{}}"),
            ("src/index.ts", "export const fixture = true;\n"),
        ],
        platforms: EVERY_PLATFORM,
    },
];

pub(crate) fn definition(language_id: &str) -> Option<&'static LanguageDefinition> {
    LANGUAGE_DEFINITIONS
        .iter()
        .find(|definition| definition.id == language_id)
}

/// Resolves an admitted source-file extension to its owning language and LSP `languageId`.
pub(crate) fn definition_for_extension(
    extension: &str,
) -> Option<(&'static LanguageDefinition, &'static str)> {
    LANGUAGE_DEFINITIONS.iter().find_map(|definition| {
        definition
            .language_id_for_extension(extension)
            .map(|language_id| (definition, language_id))
    })
}

pub(crate) fn definition_for_server(server_id: &str) -> Option<&'static LanguageDefinition> {
    LANGUAGE_DEFINITIONS
        .iter()
        .find(|definition| definition.server_id == server_id)
}

/// Named lookups for the two languages the tests exercise directly. Without them every test that
/// used to write `LanguageFamily::Rust` would carry its own `expect`, which is noise that also
/// hides which assertion actually failed.
#[cfg(test)]
pub(crate) fn rust() -> &'static LanguageDefinition {
    definition("rust").expect("rust is registered")
}

#[cfg(test)]
pub(crate) fn typescript() -> &'static LanguageDefinition {
    definition("typescript_javascript").expect("typescript/javascript is registered")
}
