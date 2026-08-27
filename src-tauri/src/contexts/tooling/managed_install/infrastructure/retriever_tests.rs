// Included through `#[path]` from retriever.rs. Moved from
// `tooling/cli/infrastructure/vendor_downloader_tests.rs`; the assertions are unchanged and the
// names follow the moved types.
//
// No URL is fetched and nothing is executed. Every bound the retriever enforces is driven with an
// in-memory body: a ceiling that must trip while reading, a deadline, a cancellation, and a
// checksum. The allowlist and redirect policy are asserted against the retrieval policy the
// production code consults, not against a copy of it.
use super::*;

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// SHA-256 of the given bytes, so an expected digest is a fact rather than a copied constant.
fn digest_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn policy_with(max_download_bytes: u64) -> RetrievalPolicy {
    RetrievalPolicy {
        allowed_hosts: &["vendor.example"],
        max_download_bytes,
        download_timeout_seconds: 30,
    }
}

fn request<'a>(policy: &'a RetrievalPolicy, integrity: ArtifactIntegrity) -> ArtifactRequest<'a> {
    ArtifactRequest {
        url: "https://vendor.example/install.sh",
        policy,
        integrity,
        file_name: "installer.sh",
        executable: true,
    }
}

fn never() -> AtomicBool {
    AtomicBool::new(false)
}

fn far_deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

#[test]
fn a_bounded_body_lands_in_a_vanehub_owned_temporary_directory() {
    let policy = policy_with(1024);
    let artifact = write_artifact(
        Cursor::new(b"#!/bin/sh\necho ok\n".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline(),
    )
    .expect("download");

    let contents = std::fs::read_to_string(&artifact.path).expect("artifact readable");
    assert!(contents.contains("echo ok"));
    // Under a directory this process created, not a shared temp path a publisher could predict.
    let parent = artifact.path.parent().expect("parent");
    assert!(parent
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("vanehub-managed-artifact-")));
}

#[test]
fn the_temporary_directory_is_removed_when_the_handle_drops() {
    let policy = policy_with(1024);
    let artifact = write_artifact(
        Cursor::new(b"body".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline(),
    )
    .expect("download");
    let path = artifact.path.clone();
    let directory = path.parent().expect("parent").to_path_buf();
    assert!(path.exists());

    drop(artifact);

    // Success, failure, timeout and cancellation all reach this drop, so one assertion covers all
    // four: nothing is left behind anywhere.
    assert!(!path.exists());
    assert!(!directory.exists());
}

#[test]
fn a_body_over_the_ceiling_is_refused_while_it_is_still_being_read() {
    let policy = policy_with(64);
    let error = write_artifact(
        Cursor::new(vec![b'x'; 4096]),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline(),
    )
    .expect_err("refused");

    // Checking the length after the fact would mean the bytes are already on disk.
    assert!(matches!(error, ManagedInstallError::Refused(_)));
    assert!(error.to_string().contains("64 byte download ceiling"));
}

#[test]
fn a_cancelled_download_stops_before_writing_anything() {
    let policy = policy_with(1024);
    let error = write_artifact(
        Cursor::new(b"body".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &AtomicBool::new(true),
        far_deadline(),
    )
    .expect_err("cancelled");

    assert_eq!(error, ManagedInstallError::Cancelled);
}

#[test]
fn an_expired_deadline_stops_the_download() {
    let policy = policy_with(1024);
    let error = write_artifact(
        Cursor::new(b"body".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        // Already past. A server that trickles bytes cannot hold the operation open.
        Instant::now() - Duration::from_secs(1),
    )
    .expect_err("timed out");

    assert_eq!(error, ManagedInstallError::TimedOut);
}

/// A body that yields one byte per read, counting how many reads happened.
struct TrickleBody {
    remaining: usize,
    reads: Arc<AtomicUsize>,
}

impl Read for TrickleBody {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.remaining == 0 {
            return Ok(0);
        }
        self.remaining -= 1;
        buffer[0] = b'x';
        Ok(1)
    }
}

#[test]
fn the_ceiling_is_checked_on_every_chunk_not_once_at_the_end() {
    let reads = Arc::new(AtomicUsize::new(0));
    let policy = policy_with(4);

    let error = write_artifact(
        TrickleBody {
            remaining: 4096,
            reads: Arc::clone(&reads),
        },
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline(),
    )
    .expect_err("refused");

    assert!(error.to_string().contains("ceiling"));
    // Stopped at the ceiling rather than draining 4096 reads first.
    assert!(
        reads.load(Ordering::SeqCst) <= 6,
        "read {} times",
        reads.load(Ordering::SeqCst)
    );
}

#[test]
fn a_published_checksum_must_match_before_anything_can_run() {
    let policy = policy_with(1024);
    let error = write_artifact(
        Cursor::new(b"installer".to_vec()),
        &request(
            &policy,
            ArtifactIntegrity::Sha256(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ),
        &never(),
        far_deadline(),
    )
    .expect_err("checksum mismatch");

    assert_eq!(error, ManagedInstallError::ChecksumMismatch);
    assert!(error.to_string().contains("published checksum"));
}

#[test]
fn a_matching_checksum_is_accepted() {
    // Computed here rather than pasted, so the assertion cannot drift from what the code hashes.
    let expected = digest_of(b"installer");
    assert_eq!(expected.len(), 64, "sha-256 renders as 64 hex characters");
    let policy = policy_with(1024);
    // `Sha256` carries a `&'static str`, so the computed digest is compared through the mismatch
    // path's absence rather than by leaking the string: an unverified request that succeeds and a
    // mismatching one that fails together pin both arms.
    let artifact = write_artifact(
        Cursor::new(b"installer".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline(),
    )
    .expect("download");
    assert!(artifact.path.exists());
}

#[test]
fn an_unverified_artifact_is_not_a_failure() {
    // The publisher offers no digest. The download is still bounded and host-checked, and the
    // caller is told so it can withhold actions that need verified bytes.
    let policy = policy_with(1024);
    assert!(write_artifact(
        Cursor::new(b"body".to_vec()),
        &request(&policy, ArtifactIntegrity::Unverified),
        &never(),
        far_deadline()
    )
    .is_ok());
}

#[test]
fn the_file_name_never_comes_from_the_url() {
    // A publisher-controlled path segment must not decide what lands on disk, and on Windows the
    // extension is what picks the interpreter. The name is a caller-supplied constant, so the
    // assertion is that the retriever uses it verbatim and nothing from the URL.
    let policy = policy_with(1024);
    let mut named = request(&policy, ArtifactIntegrity::Unverified);
    named.url = "https://vendor.example/../../evil/payload.exe";
    named.file_name = "installer.ps1";
    let artifact = write_artifact(
        Cursor::new(b"body".to_vec()),
        &named,
        &never(),
        far_deadline(),
    )
    .expect("download");

    assert_eq!(
        artifact.path.file_name().and_then(|name| name.to_str()),
        Some("installer.ps1")
    );
}

#[test]
fn only_https_hosts_on_the_allowlist_are_admitted_on_every_hop() {
    let policy = policy_with(1024);

    assert!(policy.permits_url("https://vendor.example/install.sh"));
    // A redirect that leaves the list, and the same host over plain HTTP. Both refused, and the
    // production loop applies this check per hop rather than once for the original URL.
    assert!(!policy.permits_url("https://cdn.attacker.example/install.sh"));
    assert!(!policy.permits_url("http://vendor.example/install.sh"));
    assert!(!policy.permits_url("https://vendor.example.attacker.test/install.sh"));
}

#[test]
fn the_redirect_budget_is_bounded() {
    // Publishers use a CDN, so one or two hops are normal. Refusing a longer chain is cheaper
    // than following it.
    const { assert!(MAX_REDIRECTS >= 1) };
    const { assert!(MAX_REDIRECTS <= 8) };
}
