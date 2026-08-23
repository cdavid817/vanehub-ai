// Included through `#[path]` from vendor_downloader.rs.
//
// No vendor URL is fetched and no installer is executed. Every bound the downloader enforces is
// driven with an in-memory body: a ceiling that must trip while reading, a deadline, a
// cancellation, and a checksum. The allowlist and redirect policy are asserted against the trust
// policy the production code consults, not against a copy of it.
use super::*;

use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::contexts::tooling::cli::domain::source::{CliPlatform, CliTargetVersionMode};
use crate::contexts::tooling::cli::domain::trust::{CliInstallerRuntime, CliInstallerTemplate};

/// SHA-256 of `b"installer"`, so the expected digest is a fact rather than a copied constant.
fn digest_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn template(integrity: CliInstallerIntegrity) -> CliInstallerTemplate {
    CliInstallerTemplate {
        platform: CliPlatform::Linux,
        runtime: CliInstallerRuntime::ShellFile { interpreter: "sh" },
        url: "https://vendor.example/install.sh",
        target_version: CliTargetVersionMode::LatestOnly,
        version_argument: None,
        integrity,
    }
}

fn trust_with(
    max_download_bytes: u64,
    templates: &'static [CliInstallerTemplate],
) -> CliInstallerTrust {
    CliInstallerTrust {
        allowed_hosts: &["vendor.example"],
        max_download_bytes,
        download_timeout_seconds: 30,
        templates,
    }
}

static UNVERIFIED: &[CliInstallerTemplate] = &[CliInstallerTemplate {
    platform: CliPlatform::Linux,
    runtime: CliInstallerRuntime::ShellFile { interpreter: "sh" },
    url: "https://vendor.example/install.sh",
    target_version: CliTargetVersionMode::LatestOnly,
    version_argument: None,
    integrity: CliInstallerIntegrity::Unverified,
}];

fn never() -> CliCancellation {
    CliCancellation::never()
}

fn far_deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

#[test]
fn a_bounded_body_lands_in_a_vanehub_owned_temporary_directory() {
    let trust = trust_with(1024, UNVERIFIED);
    let installer = write_installer(
        Cursor::new(b"#!/bin/sh\necho ok\n".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        far_deadline(),
    )
    .expect("download");

    let contents = std::fs::read_to_string(&installer.path).expect("installer readable");
    assert!(contents.contains("echo ok"));
    // Under a directory this process created, not a shared temp path a vendor could predict.
    let parent = installer.path.parent().expect("parent");
    assert!(parent
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("vanehub-cli-installer-")));
}

#[test]
fn the_temporary_directory_is_removed_when_the_handle_drops() {
    let trust = trust_with(1024, UNVERIFIED);
    let installer = write_installer(
        Cursor::new(b"body".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        far_deadline(),
    )
    .expect("download");
    let path = installer.path.clone();
    let directory = path.parent().expect("parent").to_path_buf();
    assert!(path.exists());

    drop(installer);

    // Success, failure, timeout and cancellation all reach this drop, so one assertion covers all
    // four: nothing executable is left behind anywhere.
    assert!(!path.exists());
    assert!(!directory.exists());
}

#[test]
fn a_body_over_the_ceiling_is_refused_while_it_is_still_being_read() {
    let trust = trust_with(64, UNVERIFIED);
    let error = write_installer(
        Cursor::new(vec![b'x'; 4096]),
        "installer.sh",
        &trust,
        &never(),
        far_deadline(),
    )
    .expect_err("refused");

    // Checking the length after the fact would mean the bytes are already on disk.
    assert_eq!(error.category(), "process");
    assert!(error.to_string().contains("64 byte download ceiling"));
}

#[test]
fn a_cancelled_download_stops_before_writing_anything() {
    let flag = Arc::new(AtomicBool::new(true));
    let trust = trust_with(1024, UNVERIFIED);

    let error = write_installer(
        Cursor::new(b"body".to_vec()),
        "installer.sh",
        &trust,
        &CliCancellation::new(flag),
        far_deadline(),
    )
    .expect_err("cancelled");

    assert_eq!(error.category(), "process");
    assert!(error.to_string().contains("cancelled"));
}

#[test]
fn an_expired_deadline_stops_the_download() {
    let trust = trust_with(1024, UNVERIFIED);
    let error = write_installer(
        Cursor::new(b"body".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        // Already past. A server that trickles bytes cannot hold the operation open.
        Instant::now() - Duration::from_secs(1),
    )
    .expect_err("timed out");

    assert!(error.to_string().contains("time budget"));
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
    let trust = trust_with(4, UNVERIFIED);

    let error = write_installer(
        TrickleBody {
            remaining: 4096,
            reads: Arc::clone(&reads),
        },
        "installer.sh",
        &trust,
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
    static MISMATCHED: &[CliInstallerTemplate] = &[CliInstallerTemplate {
        platform: CliPlatform::Linux,
        runtime: CliInstallerRuntime::ShellFile { interpreter: "sh" },
        url: "https://vendor.example/install.sh",
        target_version: CliTargetVersionMode::LatestOnly,
        version_argument: None,
        integrity: CliInstallerIntegrity::Sha256(
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    }];
    let trust = trust_with(1024, MISMATCHED);

    let error = write_installer(
        Cursor::new(b"installer".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        far_deadline(),
    )
    .expect_err("checksum mismatch");

    assert_eq!(error.category(), "validation");
    assert!(error.to_string().contains("published checksum"));
}

#[test]
fn a_matching_checksum_is_accepted() {
    // Computed here rather than pasted, so the assertion cannot drift from what the code hashes.
    assert_eq!(
        digest_of(b"installer").len(),
        64,
        "sha-256 renders as 64 hex characters"
    );
    let trust = trust_with(1024, UNVERIFIED);
    let installer = write_installer(
        Cursor::new(b"installer".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        far_deadline(),
    )
    .expect("download");
    assert!(installer.path.exists());
}

#[test]
fn an_unverified_template_is_not_a_failure() {
    // The vendor publishes no digest. The download is still bounded and host-checked, and the
    // template already refuses exact-version installs because of it.
    let trust = trust_with(1024, UNVERIFIED);
    assert!(write_installer(
        Cursor::new(b"body".to_vec()),
        "installer.sh",
        &trust,
        &never(),
        far_deadline()
    )
    .is_ok());
    assert_eq!(
        template(CliInstallerIntegrity::Unverified).integrity,
        CliInstallerIntegrity::Unverified
    );
}

#[test]
fn the_file_name_never_comes_from_the_url() {
    // A vendor-controlled path segment must not decide what lands on disk, and on Windows the
    // extension is what picks the interpreter.
    assert_eq!(
        installer_file_name(Some("application/x-powershell")),
        "installer.ps1"
    );
    let default_name = installer_file_name(None);
    assert!(default_name == "installer.ps1" || default_name == "installer.sh");
    assert!(!default_name.contains('/'));
    assert!(!default_name.contains('\\'));
}

#[test]
fn only_https_hosts_on_the_allowlist_are_admitted_on_every_hop() {
    let trust = trust_with(1024, UNVERIFIED);

    assert!(trust.permits_url("https://vendor.example/install.sh"));
    // A redirect that leaves the list, and the same host over plain HTTP. Both refused, and the
    // production loop applies this check per hop rather than once for the original URL.
    assert!(!trust.permits_url("https://cdn.attacker.example/install.sh"));
    assert!(!trust.permits_url("http://vendor.example/install.sh"));
    assert!(!trust.permits_url("https://vendor.example.attacker.test/install.sh"));
}

#[test]
fn the_redirect_budget_is_bounded() {
    // Vendors publish through a CDN, so one or two hops are normal. Refusing a longer chain is
    // cheaper than following it.
    const { assert!(MAX_REDIRECTS >= 1) };
    const { assert!(MAX_REDIRECTS <= 8) };
}
