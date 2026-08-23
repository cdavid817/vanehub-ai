use super::*;
use std::sync::Mutex;

struct Locator(PathBuf);

impl DelegationExecutableResolver for Locator {
    fn resolve(&self, _: DelegationTarget) -> Option<String> {
        Some(self.0.to_string_lossy().into_owned())
    }
}

#[derive(Default)]
struct Runner(Mutex<Vec<Vec<String>>>);

impl PassiveDelegationProbeRunner for Runner {
    fn execute(&self, _: &Path, arguments: &[&str]) -> Result<String, ()> {
        self.0
            .lock()
            .expect("calls")
            .push(arguments.iter().map(|value| (*value).to_owned()).collect());
        match arguments {
            ["--version"] => Ok("codex-cli 0.50.0".to_owned()),
            ["--help"] => Ok("exec".to_owned()),
            ["exec", "--help"] => Ok("--json --output-schema --sandbox --ephemeral".to_owned()),
            _ => Err(()),
        }
    }
}

#[test]
fn probe_uses_only_passive_version_and_help_commands_and_hashes_binary() {
    let root = tempfile::tempdir().expect("root");
    let executable = root.path().join("codex.exe");
    std::fs::write(&executable, b"reviewed cli fixture").expect("executable");
    let runner = Arc::new(Runner::default());
    let probe = PassiveDelegationProbe::with_ports(
        Arc::new(Locator(executable.canonicalize().expect("canonical"))),
        runner.clone(),
        Arc::new(|_| DelegationAuthentication::Available),
    );
    let observation = probe.probe(DelegationTarget::CodexCli).expect("probe");

    assert_eq!(observation.executable_sha256.len(), 64);
    assert_eq!(
        observation.authentication,
        DelegationAuthentication::Available
    );
    assert!(observation.help.contains("--output-schema"));
    assert_eq!(
        *runner.0.lock().expect("calls"),
        vec![vec!["--version"], vec!["--help"], vec!["exec", "--help"]]
    );
}

#[test]
fn symlink_or_missing_executable_is_rejected_before_probe() {
    let runner = Arc::new(Runner::default());
    let probe = PassiveDelegationProbe::with_ports(
        Arc::new(Locator(PathBuf::from("Z:/missing/codex.exe"))),
        runner.clone(),
        Arc::new(|_| DelegationAuthentication::Unknown),
    );
    assert!(probe.probe(DelegationTarget::CodexCli).is_err());
    assert!(runner.0.lock().expect("calls").is_empty());
}
