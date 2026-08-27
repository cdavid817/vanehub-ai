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

use crate::contexts::tooling::managed_install::api::{
    ArtifactIntegrity, ExtractionLimits, RetrievalPolicy,
};

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

/// One argument in an interpreter launch template.
///
/// Placeholders are variants rather than strings so an unresolved one is a case the compiler
/// knows about, not a substitution that quietly failed and left `{launcher}` on a command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchArgument {
    Literal(&'static str),
    /// The versioned launcher resolved inside the install directory.
    Launcher,
    /// The platform's configuration directory inside the install directory.
    ConfigurationDirectory,
    /// A writable directory unique to the workspace being served.
    WorkspaceDataDirectory,
}

/// A server that runs through a host interpreter rather than as an executable of its own.
///
/// `executables` on the owning definition names the *interpreter* candidates under this shape.
/// The server itself lives in the template, which is why an install directory rather than an
/// executable is what a user points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterpreterLaunch {
    /// What the user has to install themselves, named in the prerequisite reason. Display text,
    /// not an identifier.
    pub(crate) prerequisite: &'static str,
    /// The one directory inside the install that holds the launcher. Not searched recursively:
    /// a launcher found three levels down is not the install layout this entry describes.
    pub(crate) launcher_directory: &'static str,
    pub(crate) launcher_prefix: &'static str,
    pub(crate) launcher_suffix: &'static str,
    /// The configuration directory for each platform, relative to the install directory.
    pub(crate) configuration_directories: &'static [(HostPlatform, &'static str)],
    pub(crate) arguments: &'static [LaunchArgument],
}

impl InterpreterLaunch {
    pub(crate) fn configuration_directory(&self, platform: HostPlatform) -> Option<&'static str> {
        self.configuration_directories
            .iter()
            .find(|(declared, _)| *declared == platform)
            .map(|(_, directory)| *directory)
    }
}

/// The archive format a published distribution ships in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DistributionFormat {
    /// No registered language ships a zip today. The adapter behind it is live and tested; this
    /// is the declaration that selects it, and the first zip-published server removes the
    /// attribute rather than adding the support.
    #[expect(
        dead_code,
        reason = "no registered language ships a zip distribution yet"
    )]
    Zip,
    TarGz,
}

/// Where a language's server is published, when VaneHub can fetch it.
///
/// The bounds are `managed_install`'s own types rather than a second declaration of the same
/// three numbers -- that capability enforces them, and a copy here is what would drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PublishedDistribution {
    pub(crate) url: &'static str,
    pub(crate) policy: RetrievalPolicy,
    pub(crate) integrity: ArtifactIntegrity,
    pub(crate) format: DistributionFormat,
    pub(crate) extraction: ExtractionLimits,
    /// The directory inside the extracted archive that is the install root, when the archive nests
    /// everything under one. `None` when the archive's own root is the install root.
    pub(crate) root_inside_archive: Option<&'static str>,
}

impl PublishedDistribution {
    /// Whether the bytes are checked against a published digest. Reported to the surface offering
    /// the install, because an unverified download is something a user should be told about
    /// rather than something that hides behind a button.
    pub(crate) const fn is_verified(&self) -> bool {
        matches!(self.integrity, ArtifactIntegrity::Sha256(_))
    }
}

/// How a language's server is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchShape {
    /// The server is one of the declared executables, started with the declared arguments. What
    /// every language registered before Java uses, and what a manual override names a file for.
    Executable,
    Interpreter(&'static InterpreterLaunch),
}

impl LaunchShape {
    pub(crate) const fn interpreter(self) -> Option<&'static InterpreterLaunch> {
        match self {
            Self::Executable => None,
            Self::Interpreter(launch) => Some(launch),
        }
    }
}

pub(crate) struct LanguageDefinition {
    pub(crate) id: &'static str,
    pub(crate) server_id: &'static str,
    /// Candidate executable names in preference order. A language may ship under more than one
    /// name, and the first that resolves wins.
    pub(crate) executables: &'static [&'static str],
    /// Decides what `executables` names and what a manual override means. Adding this field to the
    /// four entries that predate it changed nothing: they all declare `Executable`.
    pub(crate) launch: LaunchShape,
    pub(crate) default_startup_arguments: &'static [&'static str],
    /// Any one of these identifies a project root. A marker may name a path inside the candidate
    /// directory rather than a file directly in it. Order is not meaningful.
    pub(crate) root_markers: &'static [&'static str],
    /// When set, a workspace with no marker is refused instead of falling back to the session
    /// root. Only for a server that answers confidently wrong without its project metadata, which
    /// is worse than answering "unavailable".
    pub(crate) requires_root_marker: bool,
    pub(crate) extensions: &'static [ExtensionMapping],
    pub(crate) fixture_files: &'static [FixtureFile],
    pub(crate) platforms: &'static [HostPlatform],
    /// Where VaneHub can fetch this server, when it can. `None` for every language whose server
    /// is one line with a package manager the user already has -- wrapping those would add a
    /// second way to do something that already works.
    pub(crate) distribution: Option<PublishedDistribution>,
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
        launch: LaunchShape::Executable,
        default_startup_arguments: &[],
        root_markers: &["Cargo.toml"],
        requires_root_marker: false,
        extensions: &[("rs", "rust")],
        fixture_files: &[
            (
                "Cargo.toml",
                "[package]\nname='lsp_test'\nversion='0.1.0'\n",
            ),
            ("src/lib.rs", "pub fn fixture() {}\n"),
        ],
        platforms: EVERY_PLATFORM,
        distribution: None,
    },
    LanguageDefinition {
        id: "typescript_javascript",
        server_id: "typescript_language_server",
        executables: &["typescript-language-server"],
        launch: LaunchShape::Executable,
        default_startup_arguments: &["--stdio"],
        root_markers: &["tsconfig.json", "jsconfig.json", "package.json"],
        requires_root_marker: false,
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
        distribution: None,
    },
    LanguageDefinition {
        id: "go",
        server_id: "gopls",
        executables: &["gopls"],
        launch: LaunchShape::Executable,
        default_startup_arguments: &[],
        root_markers: &["go.mod"],
        requires_root_marker: false,
        extensions: &[("go", "go")],
        fixture_files: &[
            ("go.mod", "module lsp_test\n\ngo 1.21\n"),
            ("main.go", "package main\n\nfunc main() {}\n"),
        ],
        platforms: EVERY_PLATFORM,
        distribution: None,
    },
    LanguageDefinition {
        id: "python",
        server_id: "pyright",
        // The fork first: installing it is a deliberate act in a way that installing the upstream
        // server is not, so a host carrying both most likely wants it. Discovery reports which
        // candidate it selected, so the choice is visible rather than silent.
        executables: &["basedpyright-langserver", "pyright-langserver"],
        launch: LaunchShape::Executable,
        default_startup_arguments: &["--stdio"],
        root_markers: &[
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "requirements.txt",
        ],
        requires_root_marker: false,
        extensions: &[("py", "python"), ("pyi", "python")],
        fixture_files: &[
            (
                "pyproject.toml",
                "[project]\nname = \"lsp_test\"\nversion = \"0.1.0\"\n",
            ),
            ("main.py", "def fixture() -> None:\n    return None\n"),
        ],
        platforms: EVERY_PLATFORM,
        distribution: None,
    },
    LanguageDefinition {
        id: "cpp",
        server_id: "clangd",
        executables: &["clangd"],
        launch: LaunchShape::Executable,
        default_startup_arguments: &[],
        root_markers: &["compile_commands.json", "build/compile_commands.json"],
        // Without a compilation database clangd assumes default flags and then answers definitions
        // and diagnostics that are confidently wrong. An unavailable result is the better answer.
        requires_root_marker: true,
        extensions: &[
            ("c", "c"),
            // Shared by both dialects and undecidable from the extension alone. clangd infers the
            // real one from the compilation database; `c` is the more conservative hint.
            ("h", "c"),
            ("cpp", "cpp"),
            ("cc", "cpp"),
            ("cxx", "cpp"),
            ("hpp", "cpp"),
            ("hh", "cpp"),
            ("hxx", "cpp"),
        ],
        fixture_files: &[
            ("compile_commands.json", "[]"),
            ("main.cpp", "int main() { return 0; }\n"),
        ],
        platforms: EVERY_PLATFORM,
        distribution: None,
    },
    LanguageDefinition {
        id: "java",
        server_id: "jdtls",
        // The JVM, not the server. Under `Interpreter` this is what gets executed, and the server
        // is the launcher jar the template names.
        executables: &["java"],
        launch: LaunchShape::Interpreter(&JDTLS_LAUNCH),
        // The template is not a default the user replaces. Anything configured here is appended
        // after it, which is why this is empty rather than carrying the template.
        default_startup_arguments: &[],
        root_markers: &[
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
        ],
        // Unlike clangd, jdtls without project metadata degrades to single-file analysis rather
        // than to confident wrong answers, so a workspace without a marker is still worth serving.
        requires_root_marker: false,
        extensions: &[("java", "java")],
        fixture_files: &[
            (
                "pom.xml",
                concat!(
                    "<project><modelVersion>4.0.0</modelVersion>",
                    "<groupId>lsp</groupId><artifactId>lsp_test</artifactId>",
                    "<version>0.1.0</version></project>\n"
                ),
            ),
            (
                "src/main/java/Fixture.java",
                "public class Fixture { void fixture() {} }\n",
            ),
        ],
        platforms: EVERY_PLATFORM,
        distribution: Some(JDTLS_DISTRIBUTION),
    },
];

/// Where Eclipse publishes JDT Language Server.
///
/// **The bytes are not checksum-verified.** Eclipse publishes a `latest` tarball, and there is no
/// digest that stays valid across releases: pinning one means the install breaks the next time
/// they publish, and a checksum that fails for the expected reason teaches a reader to ignore
/// checksums. What still applies is everything else -- HTTPS, an exact-host allowlist checked on
/// every redirect hop, a byte ceiling enforced while reading, a deadline, cancellation, and
/// bounded extraction. The surface offering the install says so before the user clicks.
static JDTLS_DISTRIBUTION: PublishedDistribution = PublishedDistribution {
    url: "https://download.eclipse.org/jdtls/snapshots/jdt-language-server-latest.tar.gz",
    policy: RetrievalPolicy {
        allowed_hosts: &["download.eclipse.org"],
        // A real jdtls release is around 60 MB; the ceiling is generous enough not to trip on a
        // larger one and small enough that a redirected download of something else does not run.
        max_download_bytes: 256 * 1024 * 1024,
        download_timeout_seconds: 600,
    },
    integrity: ArtifactIntegrity::Unverified,
    format: DistributionFormat::TarGz,
    extraction: ExtractionLimits {
        max_total_bytes: 512 * 1024 * 1024,
        max_entries: 8_192,
    },
    // The tarball unpacks its contents at the archive root rather than under a versioned folder.
    root_inside_archive: None,
};

/// How Eclipse JDT Language Server is launched.
///
/// `java -jar <install>/plugins/org.eclipse.equinox.launcher_<version>.jar -configuration
/// <install>/config_<platform> -data <per-workspace>`. The version in the launcher's file name is
/// why it is resolved rather than declared, and the per-workspace data directory is why the
/// template is resolved at launch rather than at discovery.
static JDTLS_LAUNCH: InterpreterLaunch = InterpreterLaunch {
    prerequisite: "Java 17 or newer",
    launcher_directory: "plugins",
    launcher_prefix: "org.eclipse.equinox.launcher_",
    launcher_suffix: ".jar",
    configuration_directories: &[
        (HostPlatform::Windows, "config_win"),
        (HostPlatform::Macos, "config_mac"),
        (HostPlatform::Linux, "config_linux"),
    ],
    arguments: &[
        LaunchArgument::Literal("-Declipse.application=org.eclipse.jdt.ls.core.id1"),
        LaunchArgument::Literal("-Dosgi.bundles.defaultStartLevel=4"),
        LaunchArgument::Literal("-Declipse.product=org.eclipse.jdt.ls.core.product"),
        LaunchArgument::Literal("--add-modules=ALL-SYSTEM"),
        LaunchArgument::Literal("--add-opens"),
        LaunchArgument::Literal("java.base/java.util=ALL-UNNAMED"),
        LaunchArgument::Literal("--add-opens"),
        LaunchArgument::Literal("java.base/java.lang=ALL-UNNAMED"),
        LaunchArgument::Literal("-jar"),
        LaunchArgument::Launcher,
        LaunchArgument::Literal("-configuration"),
        LaunchArgument::ConfigurationDirectory,
        LaunchArgument::Literal("-data"),
        LaunchArgument::WorkspaceDataDirectory,
    ],
};

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

#[cfg(test)]
pub(crate) fn go() -> &'static LanguageDefinition {
    definition("go").expect("go is registered")
}

#[cfg(test)]
pub(crate) fn python() -> &'static LanguageDefinition {
    definition("python").expect("python is registered")
}

#[cfg(test)]
pub(crate) fn cpp() -> &'static LanguageDefinition {
    definition("cpp").expect("c/c++ is registered")
}

#[cfg(test)]
pub(crate) fn java() -> &'static LanguageDefinition {
    definition("java").expect("java is registered")
}
