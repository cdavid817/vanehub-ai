//! Moved from `tooling/cli/domain/trust.rs` with the code they cover. The assertions are
//! unchanged: a behavior-preserving move means these still pass without being edited to.

use super::policy::{ArtifactIntegrity, ManagedPlatform, RetrievalPolicy};

const CLAUDE: RetrievalPolicy = RetrievalPolicy {
    allowed_hosts: &["claude.ai"],
    max_download_bytes: 4 * 1024 * 1024,
    download_timeout_seconds: 60,
};

#[test]
fn only_https_urls_on_the_allowlist_are_admissible() {
    assert!(CLAUDE.permits_url("https://claude.ai/install.sh"));
    assert!(CLAUDE.permits_url("https://CLAUDE.AI/install.sh"));

    // Plain HTTP, even to an allowed host.
    assert!(!CLAUDE.permits_url("http://claude.ai/install.sh"));
    // A host that merely ends with an allowed one.
    assert!(!CLAUDE.permits_url("https://evil-claude.ai/install.sh"));
    assert!(!CLAUDE.permits_url("https://claude.ai.evil.test/install.sh"));
    // Userinfo disguising the real host.
    assert!(!CLAUDE.permits_url("https://claude.ai@evil.test/install.sh"));
    // A different host entirely, which is what a redirect check must reject.
    assert!(!CLAUDE.permits_url("https://cdn.example.test/install.sh"));
    assert!(!CLAUDE.permits_url("file:///tmp/install.sh"));
    assert!(!CLAUDE.permits_url(""));
}

#[test]
fn a_port_suffix_does_not_bypass_the_host_match() {
    assert!(CLAUDE.permits_url("https://claude.ai:443/install.sh"));
    assert!(!CLAUDE.permits_url("https://evil.test:443/install.sh"));
}

#[test]
fn a_policy_without_bounds_is_not_a_policy() {
    // The spec requires a declaration that omits an allowlist or a ceiling to be refused at
    // declaration. This is the predicate each contributor's catalog test applies.
    assert!(CLAUDE.is_bounded());
    assert!(!RetrievalPolicy {
        allowed_hosts: &[],
        ..CLAUDE
    }
    .is_bounded());
    assert!(!RetrievalPolicy {
        max_download_bytes: 0,
        ..CLAUDE
    }
    .is_bounded());
    assert!(!RetrievalPolicy {
        download_timeout_seconds: 0,
        ..CLAUDE
    }
    .is_bounded());
}

#[test]
fn the_current_platform_is_one_of_the_three_modelled_or_none() {
    let current = ManagedPlatform::current();
    if cfg!(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    )) {
        assert!(current.is_some());
    } else {
        // Never "assume Linux": an unmodelled target authorizes no platform-specific artifact.
        assert_eq!(current, None);
    }
}

#[test]
fn a_declared_digest_is_carried_verbatim() {
    let digest = "0000000000000000000000000000000000000000000000000000000000000000";
    assert!(matches!(
        ArtifactIntegrity::Sha256(digest),
        ArtifactIntegrity::Sha256(value) if value.len() == 64
    ));
    assert_ne!(
        ArtifactIntegrity::Unverified,
        ArtifactIntegrity::Sha256(digest)
    );
}
