//! Fetching an artifact VaneHub is about to run or unpack.
//!
//! Moved from `tooling/cli/infrastructure/vendor_downloader.rs`. This is the one place VaneHub
//! downloads a program it is about to use, so every bound is explicit and none of them is
//! optional:
//!
//! - **HTTPS and an allowlist.** The initial URL and every redirect target are checked against the
//!   retrieval policy's host list. A redirect that leaves the list is refused, not followed.
//! - **Redirects are followed manually.** `reqwest`'s own policy would decide where to go; here the
//!   client is built with redirects disabled and each hop is admitted by name, which is the only
//!   way the allowlist can actually apply to a hop the publisher chose.
//! - **A byte ceiling enforced while reading**, not after. Checking the length afterwards means the
//!   bytes are already on disk.
//! - **A deadline**, checked between hops and while streaming, so a server that trickles bytes
//!   cannot hold the operation open past the policy's timeout.
//! - **Cancellation**, checked at the same points.
//! - **Checksum before use.** An artifact that publishes a SHA-256 is verified against it and the
//!   file is discarded on mismatch, before anything is executed or extracted.
//!
//! The file lands in a VaneHub-owned temporary directory that is removed when the handle drops --
//! after success, failure, timeout, and cancellation alike.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::contexts::tooling::managed_install::domain::error::ManagedInstallError;
use crate::contexts::tooling::managed_install::domain::policy::{
    ArtifactIntegrity, RetrievalPolicy,
};
use crate::platform::logging::redact_text;
use crate::platform::network::blocking_no_redirect_http_client;

/// How many redirects to admit before giving up.
///
/// Publishers use a CDN, so one or two hops are normal. A chain longer than this is not a
/// publisher being tidy; refusing it is cheaper than following it.
const MAX_REDIRECTS: usize = 4;

/// Read granularity. Small enough that the ceiling and the deadline are checked often.
const CHUNK_BYTES: usize = 64 * 1024;

/// What was fetched, and the storage that owns it.
#[derive(Debug)]
pub(crate) struct RetrievedArtifact {
    pub(crate) path: PathBuf,
    /// The directory the file lives in. Held rather than used: dropping it removes the directory
    /// and everything under it, which is what makes cleanup cover an installer that wrote a
    /// sibling file next to itself.
    pub(crate) _directory: tempfile::TempDir,
}

/// What to fetch and under what bounds.
pub(crate) struct ArtifactRequest<'a> {
    pub(crate) url: &'a str,
    pub(crate) policy: &'a RetrievalPolicy,
    pub(crate) integrity: ArtifactIntegrity,
    /// The name the bytes land under. Never derived from the URL: a publisher-controlled path
    /// segment must not decide what lands on disk, and on Windows the extension is what picks an
    /// interpreter.
    pub(crate) file_name: &'a str,
    /// Whether the owner bit is set on Unix. An installer needs it; an archive about to be
    /// unpacked by this process does not.
    pub(crate) executable: bool,
}

/// Fetching an artifact under a retrieval policy.
///
/// A port so the size limit, the redirect policy, and the allowlist can be asserted without
/// reaching the network.
pub(crate) trait ManagedArtifactRetriever: Send + Sync {
    fn retrieve(
        &self,
        request: ArtifactRequest<'_>,
        cancelled: &AtomicBool,
    ) -> Result<RetrievedArtifact, ManagedInstallError>;
}

pub(crate) struct HttpsArtifactRetriever;

impl ManagedArtifactRetriever for HttpsArtifactRetriever {
    fn retrieve(
        &self,
        request: ArtifactRequest<'_>,
        cancelled: &AtomicBool,
    ) -> Result<RetrievedArtifact, ManagedInstallError> {
        let policy = request.policy;
        let deadline = Instant::now() + Duration::from_secs(policy.download_timeout_seconds);
        let client =
            blocking_no_redirect_http_client(Duration::from_secs(policy.download_timeout_seconds))
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;

        let mut current = request.url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            check_bounds(cancelled, deadline)?;
            // Checked for *this* hop, not once for the original URL. A redirect the publisher's
            // CDN chose is a URL VaneHub never audited until this line.
            if !policy.permits_url(&current) {
                return Err(ManagedInstallError::Refused(
                    "the artifact URL is not on this source's allowlist".to_string(),
                ));
            }

            let response = client
                .get(&current)
                .send()
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;

            if let Some(location) = redirect_target(&response) {
                current = location;
                continue;
            }
            if !response.status().is_success() {
                return Err(ManagedInstallError::Transfer(format!(
                    "the artifact download returned HTTP {}",
                    response.status().as_u16()
                )));
            }
            return write_artifact(response, &request, cancelled, deadline);
        }

        Err(ManagedInstallError::Refused(format!(
            "the artifact URL redirected more than {MAX_REDIRECTS} times"
        )))
    }
}

/// The `Location` of a redirect response, or `None` when this is the final hop.
fn redirect_target(response: &reqwest::blocking::Response) -> Option<String> {
    if !response.status().is_redirection() {
        return None;
    }
    response
        .headers()
        .get(reqwest::header::LOCATION)?
        .to_str()
        .ok()
        .map(str::to_string)
}

fn check_bounds(cancelled: &AtomicBool, deadline: Instant) -> Result<(), ManagedInstallError> {
    if cancelled.load(Ordering::SeqCst) {
        // Nothing has been applied yet, so this is a clean stop rather than a partial change.
        return Err(ManagedInstallError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(ManagedInstallError::TimedOut);
    }
    Ok(())
}

/// Streams the body to a VaneHub-owned temporary file under the policy's ceiling.
fn write_artifact(
    mut body: impl Read,
    request: &ArtifactRequest<'_>,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<RetrievedArtifact, ManagedInstallError> {
    // A directory rather than a bare file, so the handle owns the parent and cleanup removes both.
    let directory = tempfile::Builder::new()
        .prefix("vanehub-managed-artifact-")
        .tempdir()
        .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
    let path = directory.path().join(request.file_name);
    let mut file = std::fs::File::create(&path)
        .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;

    let mut digest = Sha256::new();
    let mut written: u64 = 0;
    let mut buffer = vec![0_u8; CHUNK_BYTES];
    loop {
        check_bounds(cancelled, deadline)?;
        let read = body
            .read(&mut buffer)
            .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
        if read == 0 {
            break;
        }
        written += read as u64;
        // Enforced while reading. Checking the length afterwards means the bytes are already here.
        if written > request.policy.max_download_bytes {
            return Err(ManagedInstallError::Refused(format!(
                "the artifact exceeded the {} byte download ceiling",
                request.policy.max_download_bytes
            )));
        }
        digest.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
    }
    file.flush()
        .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
    drop(file);

    verify_integrity(request.integrity, digest)?;
    if request.executable {
        make_executable(&path)?;
    }
    Ok(RetrievedArtifact {
        path,
        _directory: directory,
    })
}

/// Compares the streamed digest with the artifact's published one.
///
/// `Unverified` is not a failure: the publisher offers no digest, the download was still bounded
/// and host-checked, and the caller is told so it can withhold actions that need verified bytes.
fn verify_integrity(
    integrity: ArtifactIntegrity,
    digest: Sha256,
) -> Result<(), ManagedInstallError> {
    let ArtifactIntegrity::Sha256(expected) = integrity else {
        return Ok(());
    };
    let actual = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(expected) {
        // The file is removed with the temporary directory on this path, before anything uses it.
        return Err(ManagedInstallError::ChecksumMismatch);
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> Result<(), ManagedInstallError> {
    use std::os::unix::fs::PermissionsExt;
    // Owner only. The file lives in a private temporary directory and is executed by this process.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> Result<(), ManagedInstallError> {
    // Windows decides by extension, and PowerShell is invoked with `-File` against that path.
    Ok(())
}

#[cfg(test)]
#[path = "retriever_tests.rs"]
mod tests;
