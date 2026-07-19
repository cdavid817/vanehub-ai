#![allow(dead_code)]

use rusqlite::Connection;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub(crate) struct FixedClock {
    now: Arc<str>,
}

impl FixedClock {
    pub(crate) fn new(now: impl Into<String>) -> Self {
        Self {
            now: Arc::from(now.into()),
        }
    }

    pub(crate) fn now(&self) -> String {
        self.now.to_string()
    }
}

#[derive(Clone)]
pub(crate) struct SequenceIdGenerator {
    prefix: Arc<str>,
    next: Arc<AtomicU64>,
}

impl SequenceIdGenerator {
    pub(crate) fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Arc::from(prefix.into()),
            next: Arc::new(AtomicU64::new(1)),
        }
    }

    pub(crate) fn next_id(&self) -> String {
        format!(
            "{}-{}",
            self.prefix,
            self.next.fetch_add(1, Ordering::Relaxed)
        )
    }
}

#[derive(Clone)]
pub(crate) struct FakeApplicationPort<Input, Output> {
    calls: Arc<Mutex<Vec<Input>>>,
    responses: Arc<Mutex<VecDeque<Result<Output, String>>>>,
}

impl<Input, Output> FakeApplicationPort<Input, Output> {
    pub(crate) fn new(responses: impl IntoIterator<Item = Result<Output, String>>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
        }
    }

    pub(crate) fn call(&self, input: Input) -> Result<Output, String> {
        self.calls.lock().expect("fake port calls").push(input);
        self.responses
            .lock()
            .expect("fake port responses")
            .pop_front()
            .expect("fake port response")
    }

    pub(crate) fn calls(&self) -> std::sync::MutexGuard<'_, Vec<Input>> {
        self.calls.lock().expect("fake port calls")
    }
}

pub(crate) struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    pub(crate) fn new(label: &str) -> Self {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        let path = std::env::temp_dir().join(format!(
            "vanehub-native-test-{}-{}-{sequence}",
            std::process::id(),
            sanitize_label(label)
        ));
        fs::create_dir_all(&path).expect("create temporary test directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(&path, content).expect("write filesystem fixture");
        path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) struct TempSqlite {
    directory: TempDirectory,
    path: PathBuf,
}

impl TempSqlite {
    pub(crate) fn new(label: &str) -> Self {
        let directory = TempDirectory::new(label);
        let path = directory.path().join("fixture.sqlite");
        Self { directory, path }
    }

    pub(crate) fn connection(&self) -> Connection {
        let connection = Connection::open(&self.path).expect("open temporary SQLite database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("enable fixture foreign keys");
        connection
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedCommandOutput {
    pub(crate) status_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl CapturedCommandOutput {
    pub(crate) fn success(stdout: impl Into<String>) -> Self {
        Self {
            status_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub(crate) fn failure(status_code: i32, stderr: impl Into<String>) -> Self {
        Self {
            status_code: Some(status_code),
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedLogEntry {
    pub(crate) level: String,
    pub(crate) category: String,
    pub(crate) message: String,
}

#[derive(Clone, Default)]
pub(crate) struct CapturedLogSink {
    entries: Arc<Mutex<Vec<CapturedLogEntry>>>,
}

impl CapturedLogSink {
    pub(crate) fn record(
        &self,
        level: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.entries
            .lock()
            .expect("captured log entries")
            .push(CapturedLogEntry {
                level: level.into(),
                category: category.into(),
                message: message.into(),
            });
    }

    pub(crate) fn entries(&self) -> Vec<CapturedLogEntry> {
        self.entries.lock().expect("captured log entries").clone()
    }
}

fn sanitize_label(label: &str) -> String {
    let sanitized = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "fixture".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_clock_ids_and_fake_port_are_reusable() {
        let clock = FixedClock::new("2026-01-01T00:00:00Z");
        let ids = SequenceIdGenerator::new("fixture");
        let port = FakeApplicationPort::new([Ok("first"), Ok("second")]);

        assert_eq!(clock.now(), "2026-01-01T00:00:00Z");
        assert_eq!(ids.next_id(), "fixture-1");
        assert_eq!(ids.next_id(), "fixture-2");
        assert_eq!(port.call(10).expect("first response"), "first");
        assert_eq!(port.call(20).expect("second response"), "second");
        assert_eq!(*port.calls(), vec![10, 20]);
    }

    #[test]
    fn temporary_sqlite_and_filesystem_fixtures_are_isolated() {
        let database = TempSqlite::new("sqlite-support");
        let connection = database.connection();
        connection
            .execute("CREATE TABLE fixture (value TEXT NOT NULL)", [])
            .expect("create fixture table");
        connection
            .execute("INSERT INTO fixture (value) VALUES ('kept')", [])
            .expect("insert fixture value");
        let file = database
            .directory
            .write("nested/fixture.txt", "fixture content");

        assert!(database.path().exists());
        assert_eq!(
            fs::read_to_string(file).expect("read fixture"),
            "fixture content"
        );
        assert_eq!(
            connection
                .query_row("SELECT value FROM fixture", [], |row| row
                    .get::<_, String>(0))
                .expect("query fixture"),
            "kept"
        );
    }

    #[test]
    fn command_and_log_capture_keep_structured_results() {
        let success = CapturedCommandOutput::success("ready");
        let failure = CapturedCommandOutput::failure(2, "failed");
        let logs = CapturedLogSink::default();
        logs.record("info", "fixture.operation", "started");

        assert_eq!(success.status_code, Some(0));
        assert_eq!(success.stdout, "ready");
        assert_eq!(failure.status_code, Some(2));
        assert_eq!(failure.stderr, "failed");
        assert_eq!(
            logs.entries(),
            vec![CapturedLogEntry {
                level: "info".to_string(),
                category: "fixture.operation".to_string(),
                message: "started".to_string(),
            }]
        );
    }
}
