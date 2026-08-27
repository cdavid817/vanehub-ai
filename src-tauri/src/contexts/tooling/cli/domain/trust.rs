//! Trust policy for sources that execute code VaneHub fetched.
//!
//! A package manager owns its own integrity: npm verifies the registry, WinGet verifies its
//! source. A vendor installer does not -- VaneHub downloads a program and runs it, so the
//! constraints are declared here as data and enforced by the adapter.
//!
//! The rule that removes the Windows defect is structural: a template is selected by exact
//! platform match and there is no fallback arm. The previous model fell through to the shell
//! installer when no PowerShell URL existed, which produced a `bash -lc` plan on Windows for
//! `claude-code`; when that failed, a separate fallback silently switched the operation to npm, so
//! the user was told a vendor install had happened when npm had done it.

use crate::contexts::tooling::managed_install::api::{ArtifactIntegrity, RetrievalPolicy};

use super::source::{CliPlatform, CliTargetVersionMode};

/// The interpreter an installer file must be handed to. It travels with the URL because the URL
/// alone does not say: a `.sh` fed to PowerShell executes as nonsense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliInstallerRuntime {
    /// Executed as `powershell -NoProfile -ExecutionPolicy Bypass -File <downloaded>`. Never
    /// `-Command "irm ... | iex"`: a pipe-to-shell flow has nowhere to put an argument and no
    /// file to checksum.
    PowerShellFile,
    /// Executed as `<interpreter> <downloaded> [args]` after the file is written to disk.
    ShellFile { interpreter: &'static str },
}

/// How an installer accepts a version, when it accepts one at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliInstallerVersionArgument {
    /// First positional argument, as in `install.sh 1.2.3`.
    Positional,
    /// Named flag followed by the version, as in `install --version 1.2.3`.
    Flag(&'static str),
}

/// One audited installer for exactly one platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliInstallerTemplate {
    pub(crate) platform: CliPlatform,
    pub(crate) runtime: CliInstallerRuntime,
    pub(crate) url: &'static str,
    /// Version granularity this specific template honours. `LatestOnly` unless the vendor's
    /// calling convention has actually been verified -- guessing one reinstates the silent
    /// wrong-version install this field exists to prevent.
    pub(crate) target_version: CliTargetVersionMode,
    pub(crate) version_argument: Option<CliInstallerVersionArgument>,
    pub(crate) integrity: ArtifactIntegrity,
}

/// One vendor's audited installers, plus the bounds the shared retrieval runs them under.
///
/// The bounds are a `RetrievalPolicy` rather than three fields of their own: the allowlist, the
/// ceiling, and the timeout are enforced by `managed_install`, and a second declaration of them
/// here is exactly the copy this arrangement exists to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliInstallerTrust {
    pub(crate) policy: RetrievalPolicy,
    pub(crate) templates: &'static [CliInstallerTemplate],
}

impl CliInstallerTrust {
    /// The template for exactly this platform, or `None`.
    ///
    /// There is deliberately no fallback arm. On Windows this returns `None` for a vendor that
    /// publishes only a shell installer, and the caller's only correct response is to withhold the
    /// action -- not to run bash, and not to quietly substitute npm.
    pub(crate) fn template_for(&self, platform: CliPlatform) -> Option<&CliInstallerTemplate> {
        self.templates
            .iter()
            .find(|template| template.platform == platform)
    }

    pub(crate) fn template_for_current_platform(&self) -> Option<&CliInstallerTemplate> {
        CliPlatform::current().and_then(|platform| self.template_for(platform))
    }

    /// Delegates to the shared policy. Kept as a method so call sites read unchanged.
    pub(crate) fn permits_url(&self, url: &str) -> bool {
        self.policy.permits_url(url)
    }
}

/// Who is responsible for the integrity of what gets executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliSourceTrustPolicy {
    /// The package manager verifies its own artifacts; VaneHub passes explicit arguments to it.
    PackageManager,
    /// VaneHub fetches and runs an installer under the declared bounds.
    AuditedInstaller(CliInstallerTrust),
    /// VaneHub never executes anything for this source.
    DetectOnly,
}

impl CliSourceTrustPolicy {
    pub(crate) fn installer(&self) -> Option<&CliInstallerTrust> {
        match self {
            Self::AuditedInstaller(trust) => Some(trust),
            Self::PackageManager | Self::DetectOnly => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHELL_ONLY: CliInstallerTrust = CliInstallerTrust {
        policy: RetrievalPolicy {
            allowed_hosts: &["claude.ai"],
            max_download_bytes: 4 * 1024 * 1024,
            download_timeout_seconds: 60,
        },
        templates: &[
            CliInstallerTemplate {
                platform: CliPlatform::Macos,
                runtime: CliInstallerRuntime::ShellFile {
                    interpreter: "bash",
                },
                url: "https://claude.ai/install.sh",
                target_version: CliTargetVersionMode::LatestOnly,
                version_argument: None,
                integrity: ArtifactIntegrity::Unverified,
            },
            CliInstallerTemplate {
                platform: CliPlatform::Linux,
                runtime: CliInstallerRuntime::ShellFile {
                    interpreter: "bash",
                },
                url: "https://claude.ai/install.sh",
                target_version: CliTargetVersionMode::LatestOnly,
                version_argument: None,
                integrity: ArtifactIntegrity::Unverified,
            },
        ],
    };

    const CROSS_PLATFORM: CliInstallerTrust = CliInstallerTrust {
        policy: RetrievalPolicy {
            allowed_hosts: &["antigravity.google"],
            max_download_bytes: 4 * 1024 * 1024,
            download_timeout_seconds: 60,
        },
        templates: &[CliInstallerTemplate {
            platform: CliPlatform::Windows,
            runtime: CliInstallerRuntime::PowerShellFile,
            url: "https://antigravity.google/cli/install.ps1",
            target_version: CliTargetVersionMode::LatestOnly,
            version_argument: None,
            integrity: ArtifactIntegrity::Unverified,
        }],
    };

    #[test]
    fn a_shell_only_vendor_offers_nothing_on_windows() {
        // The defect: the old `platform_installer()` fell through to the `.sh` URL on Windows,
        // producing a `bash -lc` plan for a host with no POSIX shell.
        assert_eq!(SHELL_ONLY.template_for(CliPlatform::Windows), None);
        assert!(SHELL_ONLY.template_for(CliPlatform::Macos).is_some());
        assert!(SHELL_ONLY.template_for(CliPlatform::Linux).is_some());
    }

    #[test]
    fn a_windows_template_is_never_handed_to_a_shell_interpreter() {
        let windows = CROSS_PLATFORM
            .template_for(CliPlatform::Windows)
            .expect("windows template");
        assert_eq!(windows.runtime, CliInstallerRuntime::PowerShellFile);
        // And the reverse: a vendor with only a Windows template offers nothing on Unix.
        assert_eq!(CROSS_PLATFORM.template_for(CliPlatform::Macos), None);
        assert_eq!(CROSS_PLATFORM.template_for(CliPlatform::Linux), None);
    }

    #[test]
    fn template_selection_follows_the_build_target_with_no_fallback() {
        let selected = SHELL_ONLY.template_for_current_platform();
        if cfg!(target_os = "windows") {
            assert!(
                selected.is_none(),
                "windows must not select a shell template"
            );
        } else {
            assert!(selected.is_some());
        }
    }

    #[test]
    fn permits_url_reaches_the_shared_policy() {
        // The URL matrix -- scheme, suffix hosts, userinfo, port -- moved to
        // `managed_install::domain::policy_tests` with the code that decides it. Two copies of a
        // security matrix drift; what is worth asserting here is only that this accessor still
        // reaches that decision rather than growing a second one.
        assert!(SHELL_ONLY.permits_url("https://claude.ai/install.sh"));
        assert!(!SHELL_ONLY.permits_url("https://cdn.example.test/install.sh"));
        assert_eq!(
            SHELL_ONLY.permits_url("https://claude.ai@evil.test/x"),
            SHELL_ONLY
                .policy
                .permits_url("https://claude.ai@evil.test/x")
        );
    }

    #[test]
    fn only_the_audited_installer_policy_exposes_execution_bounds() {
        assert!(CliSourceTrustPolicy::PackageManager.installer().is_none());
        assert!(CliSourceTrustPolicy::DetectOnly.installer().is_none());
        let policy = CliSourceTrustPolicy::AuditedInstaller(SHELL_ONLY);
        let trust = policy.installer().expect("installer trust");
        assert_eq!(trust.policy.max_download_bytes, 4 * 1024 * 1024);
        assert_eq!(trust.policy.download_timeout_seconds, 60);
    }

    #[test]
    fn a_declared_version_convention_travels_with_its_template() {
        // Only a template whose convention has actually been verified may carry one of these, and
        // the shape differs per vendor -- `install.sh 1.2.3` versus `install --version 1.2.3`.
        let positional = CliInstallerTemplate {
            target_version: CliTargetVersionMode::Exact,
            version_argument: Some(CliInstallerVersionArgument::Positional),
            integrity: ArtifactIntegrity::Sha256(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            ..SHELL_ONLY.templates[0]
        };
        assert_eq!(
            positional.version_argument,
            Some(CliInstallerVersionArgument::Positional)
        );
        assert!(positional.target_version.accepts_exact_target());

        let flagged = CliInstallerTemplate {
            version_argument: Some(CliInstallerVersionArgument::Flag("--version")),
            ..positional
        };
        assert_eq!(
            flagged.version_argument,
            Some(CliInstallerVersionArgument::Flag("--version"))
        );
        assert_ne!(flagged.version_argument, positional.version_argument);
        assert!(matches!(
            flagged.integrity,
            ArtifactIntegrity::Sha256(digest) if digest.len() == 64
        ));
    }

    #[test]
    fn an_unverified_template_stays_latest_only() {
        // No published digest and no verified version convention means the installer may not be
        // aimed at an exact version -- it would install latest and report the requested version.
        for template in SHELL_ONLY.templates {
            assert_eq!(template.integrity, ArtifactIntegrity::Unverified);
            assert_eq!(template.target_version, CliTargetVersionMode::LatestOnly);
            assert_eq!(template.version_argument, None);
        }
    }
}
