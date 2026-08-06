//! Per-launch port/token generation and the local discovery file the hook wrapper reads
//! (design.md D3, D6). Both are regenerated every application start — nothing here is meant to
//! survive a restart.
//!
//! Resolves its own path via `dirs::data_local_dir()` rather than Tauri's `app.path()` API:
//! the hook wrapper (`src-tauri/src/bin/vanehub-permission-hook.rs`) is a standalone binary with
//! no Tauri runtime available to it at all, so both sides need a path neither depends on Tauri
//! to compute. `DISCOVERY_SUBDIR`/`DISCOVERY_FILE_NAME` are duplicated verbatim in the wrapper —
//! they must be kept in sync by hand, since the wrapper deliberately doesn't link this crate.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::PathBuf;

pub(crate) const DISCOVERY_SUBDIR: &str = "VaneHub";
pub(crate) const DISCOVERY_FILE_NAME: &str = "permission-hook.json";

#[derive(Serialize)]
struct DiscoveryFile {
    port: u16,
    token: String,
}

/// `None` only on a platform where `dirs` cannot resolve a local-data directory at all — the
/// caller treats that the same as any other best-effort startup failure (server doesn't start).
pub(crate) fn discovery_file_path() -> Option<PathBuf> {
    Some(
        dirs::data_local_dir()?
            .join(DISCOVERY_SUBDIR)
            .join(DISCOVERY_FILE_NAME),
    )
}

/// A fresh, random per-launch bearer token (`claude-code-permission-hook`'s "Loopback server is
/// authenticated and bound to localhost only"). 32 random bytes, hex encoded so it round-trips
/// safely through JSON, env vars, and command lines untouched.
pub(crate) fn generate_token() -> String {
    let bytes: [u8; 32] = std::array::from_fn(|_| rand::random::<u8>());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn write_discovery_file(path: &std::path::Path, port: u16, token: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string(&DiscoveryFile {
        port,
        token: token.to_string(),
    })
    .expect("discovery file payload is always serializable");
    fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_64_hex_characters_and_differ_each_call() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn discovery_file_path_is_scoped_under_a_vanehub_subdirectory() {
        let path = discovery_file_path().expect("dirs should resolve on the test platform");
        assert_eq!(path.file_name().unwrap(), "permission-hook.json");
        assert_eq!(
            path.parent().unwrap().file_name().unwrap(),
            "VaneHub"
        );
    }

    #[test]
    fn write_discovery_file_round_trips_through_json() {
        let path = std::env::temp_dir()
            .join(format!("vanehub-hook-discovery-test-{}", std::process::id()))
            .join("nested")
            .join("permission-hook.json");

        write_discovery_file(&path, 54321, "test-token").expect("write should succeed");
        let raw = fs::read_to_string(&path).expect("read back");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");

        assert_eq!(value["port"], 54321);
        assert_eq!(value["token"], "test-token");

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
