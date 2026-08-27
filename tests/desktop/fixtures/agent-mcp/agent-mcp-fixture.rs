use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let executable = env::current_exe().unwrap_or_else(|_| PathBuf::from("agent-mcp-fixture"));
    let agent = executable
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown");
    let script = executable
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("agent-mcp-fixture.mjs");
    let status = Command::new("node")
        .arg(script)
        .args(env::args().skip(1))
        .env("VANEHUB_MCP_FIXTURE_AGENT", agent)
        .status();
    match status {
        Ok(value) if value.success() => ExitCode::SUCCESS,
        Ok(value) => ExitCode::from(value.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to launch MCP Agent fixture: {error}");
            ExitCode::FAILURE
        }
    }
}
