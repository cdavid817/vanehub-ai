//! What bounds a download that VaneHub is about to run or unpack.
//!
//! A package manager owns its own integrity: npm verifies the registry, WinGet verifies its
//! source. Fetching an artifact directly does not come with that, so the constraints are declared
//! here as data by whoever contributes the artifact, and enforced in one place by the retriever.
//!
//! These types deliberately know nothing about CLI tools or language servers. The contributing
//! context describes what it is fetching; this describes what is permitted while fetching it.

/// The platform an artifact is published for.
///
/// Separate from any consumer's platform enum on purpose. Sharing one would make this capability
/// describe the first consumer's vocabulary, which is the drift the extraction exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedPlatform {
    Windows,
    Macos,
    Linux,
}

impl ManagedPlatform {
    /// The platform this build runs on, or `None` on a target this capability does not model.
    /// `None` means "no platform-specific artifact is authorized", never "assume Linux".
    pub(crate) fn current() -> Option<Self> {
        if cfg!(target_os = "windows") {
            Some(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Some(Self::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else {
            None
        }
    }
}

/// An integrity check the retriever must perform on the downloaded bytes before anything uses
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactIntegrity {
    /// No published digest. The download is still bounded and host-checked, but the bytes are
    /// unverified -- which is why a caller that needs verified bytes must withhold the action.
    Unverified,
    Sha256(&'static str),
}

/// Bounds and host policy every retrieval runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RetrievalPolicy {
    /// Hosts the initial URL and every redirect target must match exactly. A redirect that leaves
    /// this list is rejected rather than followed.
    pub(crate) allowed_hosts: &'static [&'static str],
    pub(crate) max_download_bytes: u64,
    pub(crate) download_timeout_seconds: u64,
}

impl RetrievalPolicy {
    /// Whether this declaration actually bounds anything. A contributor that leaves the allowlist
    /// empty or a ceiling at zero has declared no policy at all, and the retriever must not treat
    /// that as permission.
    ///
    /// Checked by a test that walks each contributor's catalog rather than by a fallible
    /// constructor: the declarations are `static` data the build already fixes, and making startup
    /// fallible over a constant would trade a compile-time-known fact for a runtime one.
    pub(crate) const fn is_bounded(&self) -> bool {
        !self.allowed_hosts.is_empty()
            && self.max_download_bytes > 0
            && self.download_timeout_seconds > 0
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
