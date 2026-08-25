//! Fetching an audited vendor installer.
//!
//! This is the one place VaneHub downloads a program it is about to run, so every bound is explicit
//! and none of them is optional:
//!
//! - **HTTPS and an allowlist.** The initial URL and every redirect target are checked against the
//!   trust policy's host list. A redirect that leaves the list is refused, not followed.
//! - **Redirects are followed manually.** `reqwest`'s own policy would decide where to go; here the
//!   client is built with redirects disabled and each hop is admitted by name, which is the only
//!   way the allowlist can actually apply to a hop the vendor chose.
//! - **A byte ceiling enforced while reading**, not after. Checking the length afterwards means the
//!   bytes are already on disk.
//! - **A deadline**, checked between hops and while streaming, so a server that trickles bytes
//!   cannot hold the operation open past the policy's timeout.
//! - **Cancellation**, checked at the same points.
//! - **Checksum before execution.** A template that publishes a SHA-256 is verified against it and
//!   the file is discarded on mismatch, before anything is executed.
//!
//! The file lands in a VaneHub-owned temporary directory that is removed when the handle drops --
//! after success, failure, timeout, and cancellation alike.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::CliCancellation;
use crate::contexts::tooling::cli::domain::trust::{CliInstallerIntegrity, CliInstallerTrust};
use crate::platform::logging::redact_text;
use crate::platform::network::blocking_no_redirect_http_client;

use super::vendor_source::{CliInstallerDownloader, DownloadedInstaller};

/// How many redirects to admit before giving up.
///
/// Vendors publish through a CDN, so one or two hops are normal. A chain longer than this is not a
/// vendor being tidy; refusing it is cheaper than following it.
const MAX_REDIRECTS: usize = 4;

/// Read granularity. Small enough that the ceiling and the deadline are checked often.
const CHUNK_BYTES: usize = 64 * 1024;

pub(crate) struct HttpsInstallerDownloader;

impl CliInstallerDownloader for HttpsInstallerDownloader {
    fn download(
        &self,
        url: &str,
        trust: &CliInstallerTrust,
        cancellation: &CliCancellation,
    ) -> Result<DownloadedInstaller, CliEnvironmentError> {
        let deadline = Instant::now() + Duration::from_secs(trust.download_timeout_seconds);
        let client =
            blocking_no_redirect_http_client(Duration::from_secs(trust.download_timeout_seconds))
                .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;

        let mut current = url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            check_bounds(cancellation, deadline)?;
            // Checked for *this* hop, not once for the original URL. A redirect the vendor's CDN
            // chose is a URL VaneHub never audited until this line.
            if !trust.permits_url(&current) {
                return Err(CliEnvironmentError::Validation(
                    "the installer URL is not on this source's allowlist".to_string(),
                ));
            }

            let response = client
                .get(&current)
                .send()
                .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;

            if let Some(location) = redirect_target(&response) {
                current = location;
                continue;
            }
            if !response.status().is_success() {
                return Err(CliEnvironmentError::Process(format!(
                    "the installer download returned HTTP {}",
                    response.status().as_u16()
                )));
            }
            let name = installer_file_name(
                response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok()),
            );
            return write_installer(response, name, trust, cancellation, deadline);
        }

        Err(CliEnvironmentError::Process(format!(
            "the installer URL redirected more than {MAX_REDIRECTS} times"
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

fn check_bounds(
    cancellation: &CliCancellation,
    deadline: Instant,
) -> Result<(), CliEnvironmentError> {
    if cancellation.is_cancelled() {
        // Nothing has been applied yet, so this is a clean stop rather than a partial change.
        return Err(CliEnvironmentError::Process(
            "the installer download was cancelled".to_string(),
        ));
    }
    if Instant::now() >= deadline {
        return Err(CliEnvironmentError::Process(
            "the installer download exceeded its time budget".to_string(),
        ));
    }
    Ok(())
}

/// Streams the body to a VaneHub-owned temporary file under the policy's ceiling.
fn write_installer(
    mut body: impl Read,
    file_name: &str,
    trust: &CliInstallerTrust,
    cancellation: &CliCancellation,
    deadline: Instant,
) -> Result<DownloadedInstaller, CliEnvironmentError> {
    // A directory rather than a bare file, so the handle owns the parent and cleanup removes both.
    let directory = tempfile::Builder::new()
        .prefix("vanehub-cli-installer-")
        .tempdir()
        .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;
    let path = directory.path().join(file_name);
    let mut file = std::fs::File::create(&path)
        .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;

    let mut digest = Sha256::new();
    let mut written: u64 = 0;
    let mut buffer = vec![0_u8; CHUNK_BYTES];
    loop {
        check_bounds(cancellation, deadline)?;
        let read = body
            .read(&mut buffer)
            .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;
        if read == 0 {
            break;
        }
        written += read as u64;
        // Enforced while reading. Checking the length afterwards means the bytes are already here.
        if written > trust.max_download_bytes {
            return Err(CliEnvironmentError::Process(format!(
                "the installer exceeded the {} byte download ceiling",
                trust.max_download_bytes
            )));
        }
        digest.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;
    }
    file.flush()
        .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))?;
    drop(file);

    verify_integrity(trust, digest)?;
    // Unix needs the bit set before the interpreter file can be handed to a shell.
    make_executable(&path)?;
    Ok(DownloadedInstaller {
        path,
        _directory: directory,
    })
}

/// Compares the streamed digest with the template's published one.
///
/// `Unverified` is not a failure: the vendor publishes no digest, the download was still bounded and
/// host-checked, and the template already refuses exact-version installs because of it.
fn verify_integrity(trust: &CliInstallerTrust, digest: Sha256) -> Result<(), CliEnvironmentError> {
    let expected = trust
        .templates
        .iter()
        .find_map(|template| match template.integrity {
            CliInstallerIntegrity::Sha256(expected) => Some(expected),
            CliInstallerIntegrity::Unverified => None,
        });
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(expected) {
        // The file is removed with the temporary directory on this path, before anything runs it.
        return Err(CliEnvironmentError::Validation(
            "the installer does not match its published checksum".to_string(),
        ));
    }
    Ok(())
}

/// A fixed name per interpreter, taken from the response's own content type where it says one.
///
/// Never a name derived from the URL: a vendor-controlled path segment must not decide what lands
/// on disk, and the extension is what Windows uses to pick an interpreter.
fn installer_file_name(content_type: Option<&str>) -> &'static str {
    let powershell = content_type.is_some_and(|value| value.contains("powershell"));
    if powershell || cfg!(target_os = "windows") {
        "installer.ps1"
    } else {
        "installer.sh"
    }
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> Result<(), CliEnvironmentError> {
    use std::os::unix::fs::PermissionsExt;
    // Owner only. The file lives in a private temporary directory and is executed by this process.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| CliEnvironmentError::Process(redact_text(&error.to_string())))
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> Result<(), CliEnvironmentError> {
    // Windows decides by extension, and PowerShell is invoked with `-File` against that path.
    Ok(())
}

#[cfg(test)]
#[path = "vendor_downloader_tests.rs"]
mod tests;
