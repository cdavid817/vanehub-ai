use crate::contexts::cli_delegation::application::{
    DelegationAuthentication, DelegationProbeObservation, DelegationProbePort, DelegationTarget,
};
use crate::contexts::tooling::cli::api::CliApi;
use crate::platform::process::{ProcessAdapter, ProcessRequest};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const MAX_PROBE_OUTPUT: usize = 256 * 1024;

pub(crate) trait PassiveDelegationProbeRunner: Send + Sync {
    fn execute(&self, executable: &Path, arguments: &[&str]) -> Result<String, ()>;
}

#[derive(Debug, Default)]
struct PlatformProbeRunner;

impl PassiveDelegationProbeRunner for PlatformProbeRunner {
    fn execute(&self, executable: &Path, arguments: &[&str]) -> Result<String, ()> {
        let output = ProcessAdapter
            .execute(
                &ProcessRequest::new(executable.as_os_str())
                    .args(arguments)
                    .timeout(Duration::from_secs(5))
                    .output_limit(MAX_PROBE_OUTPUT),
            )
            .map_err(|_| ())?;
        if !output.success() || output.output_truncated {
            return Err(());
        }
        Ok(format!("{}\n{}", output.stdout, output.stderr))
    }
}

/// Where this probe gets an executable to run.
///
/// A trait rather than `CliApi` directly so the probe can be exercised without an environment
/// service behind it. The production implementation is `CliApi`, which reads the same snapshot the
/// CLI Management page renders -- probing one installation while the page reports another is how a
/// delegation target came to be judged on a binary the user never sees.
pub(crate) trait DelegationExecutableResolver: Send + Sync {
    fn resolve(&self, target: DelegationTarget) -> Option<String>;
}

impl DelegationExecutableResolver for CliApi {
    fn resolve(&self, target: DelegationTarget) -> Option<String> {
        self.resolve_executable(target.as_str()).ok().flatten()
    }
}

pub(crate) struct PassiveDelegationProbe {
    locator: Arc<dyn DelegationExecutableResolver>,
    runner: Arc<dyn PassiveDelegationProbeRunner>,
    authentication: Arc<dyn Fn(DelegationTarget) -> DelegationAuthentication + Send + Sync>,
}

impl PassiveDelegationProbe {
    pub(crate) fn new(locator: Arc<dyn DelegationExecutableResolver>) -> Self {
        Self {
            locator,
            runner: Arc::new(PlatformProbeRunner),
            authentication: Arc::new(|_| DelegationAuthentication::Unknown),
        }
    }

    #[cfg(test)]
    fn with_ports(
        locator: Arc<dyn DelegationExecutableResolver>,
        runner: Arc<dyn PassiveDelegationProbeRunner>,
        authentication: Arc<dyn Fn(DelegationTarget) -> DelegationAuthentication + Send + Sync>,
    ) -> Self {
        Self {
            locator,
            runner,
            authentication,
        }
    }
}

impl DelegationProbePort for PassiveDelegationProbe {
    fn probe(&self, target: DelegationTarget) -> Result<DelegationProbeObservation, ()> {
        let resolved = self.locator.resolve(target).ok_or(())?;
        let executable = canonical_executable(Path::new(&resolved))?;
        let version = self.runner.execute(&executable, &["--version"])?;
        let mut help = self.runner.execute(&executable, &["--help"])?;
        if target == DelegationTarget::CodexCli {
            help.push('\n');
            help.push_str(&self.runner.execute(&executable, &["exec", "--help"])?);
        }
        Ok(DelegationProbeObservation {
            executable_sha256: hash_file(&executable)?,
            executable,
            version,
            help,
            authentication: (self.authentication)(target),
        })
    }
}

fn canonical_executable(path: &Path) -> Result<PathBuf, ()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(());
    }
    path.canonicalize().map_err(|_| ())
}

fn hash_file(path: &Path) -> Result<String, ()> {
    let mut file = std::fs::File::open(path).map_err(|_| ())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| ())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest_hex(&digest.finalize()))
}

fn digest_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
#[path = "passive_probe_tests.rs"]
mod tests;
