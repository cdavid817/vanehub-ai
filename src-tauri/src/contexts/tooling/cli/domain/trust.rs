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

/// An integrity check the adapter must perform on the downloaded bytes before executing them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliInstallerIntegrity {
    /// No published digest. The download is still bounded and host-checked, but the bytes are
    /// unverified -- which is why exact-version installs are not offered from such a template.
    Unverified,
    Sha256(&'static str),
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
    pub(crate) integrity: CliInstallerIntegrity,
}

/// Bounds and host policy shared by every audited installer download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliInstallerTrust {
    /// Hosts the initial URL and every redirect target must match exactly. A redirect that leaves
    /// this list is rejected rather than followed.
    pub(crate) allowed_hosts: &'static [&'static str],
    pub(crate) max_download_bytes: u64,
    pub(crate) download_timeout_seconds: u64,
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

    /// Whether a URL is admissible: HTTPS only, and its host must be on the allowlist. Applied to
    /// the initial URL and to every redirect target.
    pub(crate) fn permits_url(&self, url: &str) -> bool {
        let Some(rest) = url.strip_prefix("https://") else {
            return false;
        };
        // Host ends at the first `/`, `?`, or `#`. Userinfo (`user@host`) is rejected outright
        // rather than parsed: it is never needed here and is a classic way to disguise a host.
        let host = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if host.is_empty() || host.contains('@') {
            return false;
        }
        // Compare without any port suffix so `example.com:8443` cannot bypass an exact match.
        let host = host.split(':').next().unwrap_or_default();
        self.allowed_hosts
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(host))
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
        allowed_hosts: &["claude.ai"],
        max_download_bytes: 4 * 1024 * 1024,
        download_timeout_seconds: 60,
        templates: &[
            CliInstallerTemplate {
                platform: CliPlatform::Macos,
                runtime: CliInstallerRuntime::ShellFile {
                    interpreter: "bash",
                },
                url: "https://claude.ai/install.sh",
                target_version: CliTargetVersionMode::LatestOnly,
                version_argument: None,
                integrity: CliInstallerIntegrity::Unverified,
            },
            CliInstallerTemplate {
                platform: CliPlatform::Linux,
                runtime: CliInstallerRuntime::ShellFile {
                    interpreter: "bash",
                },
                url: "https://claude.ai/install.sh",
                target_version: CliTargetVersionMode::LatestOnly,
                version_argument: None,
                integrity: CliInstallerIntegrity::Unverified,
            },
        ],
    };

    const CROSS_PLATFORM: CliInstallerTrust = CliInstallerTrust {
        allowed_hosts: &["antigravity.google"],
        max_download_bytes: 4 * 1024 * 1024,
        download_timeout_seconds: 60,
        templates: &[CliInstallerTemplate {
            platform: CliPlatform::Windows,
            runtime: CliInstallerRuntime::PowerShellFile,
            url: "https://antigravity.google/cli/install.ps1",
            target_version: CliTargetVersionMode::LatestOnly,
            version_argument: None,
            integrity: CliInstallerIntegrity::Unverified,
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
    fn only_https_urls_on_the_allowlist_are_admissible() {
        assert!(SHELL_ONLY.permits_url("https://claude.ai/install.sh"));
        assert!(SHELL_ONLY.permits_url("https://CLAUDE.AI/install.sh"));

        // Plain HTTP, even to an allowed host.
        assert!(!SHELL_ONLY.permits_url("http://claude.ai/install.sh"));
        // A host that merely ends with an allowed one.
        assert!(!SHELL_ONLY.permits_url("https://evil-claude.ai/install.sh"));
        assert!(!SHELL_ONLY.permits_url("https://claude.ai.evil.test/install.sh"));
        // Userinfo disguising the real host.
        assert!(!SHELL_ONLY.permits_url("https://claude.ai@evil.test/install.sh"));
        // A different host entirely, which is what a redirect check must reject.
        assert!(!SHELL_ONLY.permits_url("https://cdn.example.test/install.sh"));
        assert!(!SHELL_ONLY.permits_url("file:///tmp/install.sh"));
        assert!(!SHELL_ONLY.permits_url(""));
    }

    #[test]
    fn a_port_suffix_does_not_bypass_the_host_match() {
        assert!(SHELL_ONLY.permits_url("https://claude.ai:443/install.sh"));
        assert!(!SHELL_ONLY.permits_url("https://evil.test:443/install.sh"));
    }

    #[test]
    fn only_the_audited_installer_policy_exposes_execution_bounds() {
        assert!(CliSourceTrustPolicy::PackageManager.installer().is_none());
        assert!(CliSourceTrustPolicy::DetectOnly.installer().is_none());
        let policy = CliSourceTrustPolicy::AuditedInstaller(SHELL_ONLY);
        let trust = policy.installer().expect("installer trust");
        assert_eq!(trust.max_download_bytes, 4 * 1024 * 1024);
        assert_eq!(trust.download_timeout_seconds, 60);
    }

    #[test]
    fn a_declared_version_convention_travels_with_its_template() {
        // Only a template whose convention has actually been verified may carry one of these, and
        // the shape differs per vendor -- `install.sh 1.2.3` versus `install --version 1.2.3`.
        let positional = CliInstallerTemplate {
            target_version: CliTargetVersionMode::Exact,
            version_argument: Some(CliInstallerVersionArgument::Positional),
            integrity: CliInstallerIntegrity::Sha256(
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
            CliInstallerIntegrity::Sha256(digest) if digest.len() == 64
        ));
    }

    #[test]
    fn an_unverified_template_stays_latest_only() {
        // No published digest and no verified version convention means the installer may not be
        // aimed at an exact version -- it would install latest and report the requested version.
        for template in SHELL_ONLY.templates {
            assert_eq!(template.integrity, CliInstallerIntegrity::Unverified);
            assert_eq!(template.target_version, CliTargetVersionMode::LatestOnly);
            assert_eq!(template.version_argument, None);
        }
    }
}
