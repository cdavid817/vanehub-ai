use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::contexts::local_media::application::ports::PythonEnvironmentDiscoveryPort;
use crate::contexts::local_media::domain::{
    PythonCompatibility, PythonDiscoveryReason, PythonDiscoverySource, PythonEnvironmentCandidate,
    PythonEnvironmentDiscovery, PythonVersion,
};
use crate::platform::process::{ProcessAdapter, ProcessRequest};

const MAX_CANDIDATES: usize = 24;
const MAX_OUTPUT_BYTES: u64 = 4 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_millis(1_500);
const PROBE_SCRIPT: &str = "import sys; print('VANE_PYTHON\\t{}\\t{}\\t{}\\t{}'.format(sys.executable, *sys.version_info[:3]))";

#[derive(Debug, Clone)]
struct CandidateSeed {
    executable: PathBuf,
    source: PythonDiscoverySource,
}

#[derive(Debug, PartialEq, Eq)]
struct ProbeIdentity {
    executable: PathBuf,
    version: PythonVersion,
}

pub(crate) struct SystemPythonEnvironmentDiscovery {
    process: ProcessAdapter,
}

impl SystemPythonEnvironmentDiscovery {
    pub(crate) fn new(process: ProcessAdapter) -> Self {
        Self { process }
    }
}

impl PythonEnvironmentDiscoveryPort for SystemPythonEnvironmentDiscovery {
    fn discover(&self, configured_paths: &[PathBuf]) -> PythonEnvironmentDiscovery {
        let mut candidates = seeds(configured_paths, &self.process)
            .into_iter()
            .filter_map(|seed| probe_seed(seed, &self.process))
            .collect::<Vec<_>>();
        deduplicate_and_sort(&mut candidates);
        PythonEnvironmentDiscovery::available(candidates)
    }
}

fn seeds(configured_paths: &[PathBuf], _process: &ProcessAdapter) -> Vec<CandidateSeed> {
    let mut result = configured_paths
        .iter()
        .take(MAX_CANDIDATES)
        .cloned()
        .map(|executable| CandidateSeed {
            executable,
            source: PythonDiscoverySource::Configured,
        })
        .collect::<Vec<_>>();

    let names: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe"]
    } else {
        &["python3", "python"]
    };
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path).filter(|entry| !entry.as_os_str().is_empty()) {
            for name in names {
                let executable = directory.join(name);
                if executable.is_file() {
                    result.push(CandidateSeed {
                        executable,
                        source: PythonDiscoverySource::Path,
                    });
                }
            }
        }
    }

    #[cfg(windows)]
    result.extend(windows_launcher_seeds(_process));

    result.truncate(MAX_CANDIDATES);
    result
}

#[cfg(windows)]
fn windows_launcher_seeds(process: &ProcessAdapter) -> Vec<CandidateSeed> {
    let Some(output) = run_bounded(process, Path::new("py.exe"), &[OsString::from("-0p")]) else {
        return Vec::new();
    };
    String::from_utf8(output)
        .ok()
        .into_iter()
        .flat_map(|text| text.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter_map(|line| windows_launcher_path(&line))
        .map(|path| CandidateSeed {
            executable: PathBuf::from(path),
            source: PythonDiscoverySource::WindowsLauncher,
        })
        .collect()
}

#[cfg(any(windows, test))]
fn windows_launcher_path(line: &str) -> Option<String> {
    let after_tag = line.trim().strip_prefix("-V:")?;
    let boundary = after_tag.find(char::is_whitespace)?;
    let raw_path = after_tag[boundary..].trim();
    let path = raw_path.strip_prefix('*').unwrap_or(raw_path).trim();
    (!path.is_empty()).then(|| path.to_string())
}

fn probe_seed(seed: CandidateSeed, process: &ProcessAdapter) -> Option<PythonEnvironmentCandidate> {
    let args = [
        OsString::from("-I"),
        OsString::from("-S"),
        OsString::from("-c"),
        OsString::from(PROBE_SCRIPT),
    ];
    let output = run_bounded(process, &seed.executable, &args)?;
    let identity = parse_probe_output(&output)?;
    let seeded = seed.executable.canonicalize().ok()?;
    let resolved = identity.executable.canonicalize().ok()?;
    if !resolved.is_file()
        || path_key(&seeded.to_string_lossy()) != path_key(&resolved.to_string_lossy())
    {
        return None;
    }
    let compatibility = identity.version.compatibility();
    Some(PythonEnvironmentCandidate {
        executable_path: resolved.to_string_lossy().into_owned(),
        version: identity.version,
        compatibility,
        reason_code: (compatibility == PythonCompatibility::Unsupported)
            .then_some(PythonDiscoveryReason::UnsupportedVersion),
        source: seed.source,
    })
}

fn run_bounded(process: &ProcessAdapter, executable: &Path, args: &[OsString]) -> Option<Vec<u8>> {
    let request = ProcessRequest::new(executable.as_os_str().to_os_string())
        .args(args.iter().cloned())
        .timeout(PROBE_TIMEOUT)
        .output_limit(MAX_OUTPUT_BYTES as usize);
    let output = process.execute(&request).ok()?;
    (output.success() && !output.output_truncated).then_some(output.stdout_bytes)
}

fn parse_probe_output(output: &[u8]) -> Option<ProbeIdentity> {
    let line = std::str::from_utf8(output).ok()?.trim();
    let mut parts = line.split('\t');
    if parts.next()? != "VANE_PYTHON" {
        return None;
    }
    let executable = PathBuf::from(parts.next()?);
    let version = PythonVersion {
        major: parts.next()?.parse().ok()?,
        minor: parts.next()?.parse().ok()?,
        patch: parts.next()?.parse().ok()?,
    };
    (parts.next().is_none() && executable.is_absolute()).then_some(ProbeIdentity {
        executable,
        version,
    })
}

fn path_key(path: &str) -> String {
    if cfg!(windows) {
        path.replace('\\', "/").to_lowercase()
    } else {
        path.to_string()
    }
}

fn deduplicate_and_sort(candidates: &mut Vec<PythonEnvironmentCandidate>) {
    let mut unique = HashMap::<String, PythonEnvironmentCandidate>::new();
    for candidate in candidates.drain(..) {
        let key = path_key(&candidate.executable_path);
        match unique.get(&key) {
            Some(existing) if existing.source.priority() <= candidate.source.priority() => {}
            _ => {
                unique.insert(key, candidate);
            }
        }
    }
    candidates.extend(unique.into_values());
    candidates.sort_by(|left, right| {
        compatibility_rank(left.compatibility)
            .cmp(&compatibility_rank(right.compatibility))
            .then_with(|| right.version.major.cmp(&left.version.major))
            .then_with(|| right.version.minor.cmp(&left.version.minor))
            .then_with(|| right.version.patch.cmp(&left.version.patch))
            .then_with(|| path_key(&left.executable_path).cmp(&path_key(&right.executable_path)))
    });
}

fn compatibility_rank(value: PythonCompatibility) -> u8 {
    match value {
        PythonCompatibility::Compatible => 0,
        PythonCompatibility::Unsupported => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        path: &str,
        minor: u16,
        source: PythonDiscoverySource,
    ) -> PythonEnvironmentCandidate {
        let version = PythonVersion {
            major: 3,
            minor,
            patch: 1,
        };
        let compatibility = version.compatibility();
        PythonEnvironmentCandidate {
            executable_path: path.to_string(),
            version,
            compatibility,
            reason_code: (compatibility == PythonCompatibility::Unsupported)
                .then_some(PythonDiscoveryReason::UnsupportedVersion),
            source,
        }
    }

    #[test]
    fn parses_only_the_structured_absolute_identity() {
        let path = if cfg!(windows) {
            "C:\\Python\\python.exe"
        } else {
            "/usr/bin/python3"
        };
        let output = format!("VANE_PYTHON\t{path}\t3\t12\t4\n");
        assert_eq!(
            parse_probe_output(output.as_bytes()).map(|value| value.version.minor),
            Some(12)
        );
        assert!(parse_probe_output(b"Python 3.12.4").is_none());
        assert!(parse_probe_output(b"VANE_PYTHON\trelative/python\t3\t12\t4").is_none());
    }

    #[test]
    fn deduplicates_with_configured_source_precedence_and_stable_order() {
        let mut values = vec![
            candidate("/b/python", 14, PythonDiscoverySource::Path),
            candidate("/a/python", 11, PythonDiscoverySource::Path),
            candidate("/a/python", 11, PythonDiscoverySource::Configured),
            candidate("/c/python", 13, PythonDiscoverySource::Path),
        ];
        deduplicate_and_sort(&mut values);
        assert_eq!(
            values
                .iter()
                .map(|value| value.executable_path.as_str())
                .collect::<Vec<_>>(),
            vec!["/c/python", "/a/python", "/b/python"]
        );
        assert_eq!(values[1].source, PythonDiscoverySource::Configured);
    }

    #[test]
    fn parses_windows_launcher_paths_without_losing_spaces() {
        assert_eq!(
            windows_launcher_path(" -V:3.12 * C:\\Program Files\\Python312\\python.exe"),
            Some("C:\\Program Files\\Python312\\python.exe".to_string()),
        );
        assert!(windows_launcher_path("not a launcher row").is_none());
    }

    #[test]
    fn configured_paths_are_seeded_first_and_bounded() {
        let configured = (0..40)
            .map(|index| PathBuf::from(format!("/configured/python-{index}")))
            .collect::<Vec<_>>();
        let result = seeds(&configured, &ProcessAdapter);
        assert!(result.len() <= MAX_CANDIDATES);
        assert_eq!(result[0].source, PythonDiscoverySource::Configured);
        assert_eq!(result[0].executable, configured[0]);
    }

    #[test]
    fn a_missing_or_non_python_candidate_is_ignored() {
        assert!(probe_seed(
            CandidateSeed {
                executable: PathBuf::from("/definitely/missing/vanehub-python"),
                source: PythonDiscoverySource::Configured,
            },
            &ProcessAdapter,
        )
        .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_candidate_claiming_a_different_executable_identity_is_ignored() {
        let claimed = tempfile::NamedTempFile::new().expect("claimed executable");
        let body = format!(
            "printf 'VANE_PYTHON\\t{}\\t3\\t12\\t1\\n'",
            claimed.path().display()
        );
        let seed = executable_fixture(&body);
        assert!(probe_seed(
            CandidateSeed {
                executable: seed.path().to_path_buf(),
                source: PythonDiscoverySource::Configured,
            },
            &ProcessAdapter,
        )
        .is_none());
    }

    #[cfg(unix)]
    fn executable_fixture(body: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let mut fixture = tempfile::NamedTempFile::new().expect("fixture");
        writeln!(fixture, "#!/bin/sh\n{body}").expect("write fixture");
        std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o700))
            .expect("make executable");
        fixture
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_rejects_timeout_and_oversized_output() {
        let timeout = executable_fixture("sleep 2");
        assert!(run_bounded(&ProcessAdapter, timeout.path(), &[]).is_none());

        let oversized = executable_fixture("head -c 5000 /dev/zero");
        assert!(run_bounded(&ProcessAdapter, oversized.path(), &[]).is_none());
    }
}
