//! Reading already-redacted records back out of the unified log files.
//!
//! This is the repair path's only view of the corpus, and the export path's only source. It never
//! writes, never un-redacts, and never invents a value a line did not carry: a record that cannot
//! satisfy the safe schema is counted as rejected rather than repaired into something plausible.

use crate::contexts::operations::application::{
    IndexedLogLevel, LineRejections, LogCorrelation, LogSourceIdentity, LogSourceSnapshot,
    OperationsLogError, RedactedLogBatch, RedactedLogRecord, RedactedLogSourceReader,
};
use crate::platform::logging::{active_file_id, directory_generation, LogEntry, LOG_FILE_NAME};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const SESSION_KEY: &str = "sessionId";
const RUN_KEY: &str = "runId";
const TRACE_KEY: &str = "traceId";
const SPAN_KEY: &str = "spanId";
const OPERATION_KEY: &str = "operationId";
const AGENT_KEY: &str = "agentId";
const SEAT_KEY: &str = "seatId";

pub(crate) struct UnifiedLogSourceReader {
    log_dir: PathBuf,
}

impl UnifiedLogSourceReader {
    pub(crate) fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }

    fn path_for(&self, source: &LogSourceIdentity) -> Option<PathBuf> {
        self.files()
            .into_iter()
            .find(|path| self.identity(path).file_id == source.file_id)
    }

    /// Every readable log file under the current directory, oldest first.
    ///
    /// Rotated files sort before the active one because their names carry the rotation stamp, and
    /// the active file has none. Reading oldest first is what makes the index fill in the order a
    /// reader would page through it.
    fn files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.log_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("vanehub") && name.ends_with(".log"))
            })
            .collect();
        files.sort_by_key(|path| {
            let is_active = path.file_name().and_then(|name| name.to_str()) == Some(LOG_FILE_NAME);
            (is_active, path.to_string_lossy().to_string())
        });
        files
    }

    fn identity(&self, path: &Path) -> LogSourceIdentity {
        LogSourceIdentity {
            directory_generation: directory_generation(&self.log_dir),
            file_id: active_file_id(path),
        }
    }
}

/// A deterministic id for a line written before ids existed.
///
/// Four things go in, and each rules out a way two different records could collapse into one:
///
/// - the **source generation**, so the same text in two files is two records;
/// - the **start offset**, so the same text twice in one file is two records — a retry loop logging
///   one failure inside a millisecond produces exactly that;
/// - the **end offset**, so a line that was extended rather than replaced is a different record
///   than the prefix that used to sit there;
/// - a **fingerprint of the already-redacted line**, so a record that moved is not mistaken for the
///   unrelated record that now occupies its offset.
///
/// Timestamp plus message is deliberately absent as an identity: it is exactly the pair that
/// repeats.
fn legacy_record_id(source: &LogSourceIdentity, start: u64, end: u64, line: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // Length-prefixed so two different splits of the same bytes cannot hash alike.
    for part in [source.as_key().as_bytes(), line.as_bytes()] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    hasher.update(start.to_be_bytes());
    hasher.update(end.to_be_bytes());
    let digest = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("legacy:v1:{digest}")
}

fn correlation(context: &BTreeMap<String, String>) -> LogCorrelation {
    let read = |key: &str| context.get(key).filter(|value| !value.is_empty()).cloned();
    LogCorrelation {
        session_id: read(SESSION_KEY),
        run_id: read(RUN_KEY),
        trace_id: read(TRACE_KEY),
        span_id: read(SPAN_KEY),
        operation_id: read(OPERATION_KEY),
        agent_id: read(AGENT_KEY),
        seat_id: read(SEAT_KEY),
    }
}

fn to_record(
    source: &LogSourceIdentity,
    start: u64,
    end: u64,
    line: &str,
) -> Option<RedactedLogRecord> {
    let entry: LogEntry = serde_json::from_str(line).ok()?;
    let occurred_at_ms = chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
        .map(|value| value.timestamp_millis())
        .unwrap_or_default();
    Some(RedactedLogRecord {
        record_id: entry
            .record_id
            .clone()
            .unwrap_or_else(|| legacy_record_id(source, start, end, line)),
        source: source.clone(),
        source_offset: start,
        occurred_at: entry.timestamp,
        occurred_at_ms,
        level: IndexedLogLevel::parse(entry.level.token()).unwrap_or(IndexedLogLevel::Info),
        category: entry.category,
        message: entry.message,
        correlation: correlation(&entry.context),
        context: entry.context,
    })
}

/// How many bytes one log line may occupy before it is skipped rather than read.
///
/// Generous for a log line and small next to the memory a process can lose to one. The bound is
/// not about typical lines at all — it is about the atypical one: a producer that concatenates a
/// payload it should have redacted away, or a file whose newline was lost to a partial write, in
/// which case "the line" is the rest of the file. Without a ceiling the indexer's memory is
/// decided by the worst line anyone ever wrote.
pub(crate) const MAX_LOG_LINE_BYTES: u64 = 1024 * 1024;

/// How much is discarded per read while skipping past an oversized line.
const SKIP_CHUNK_BYTES: usize = 64 * 1024;

enum BoundedLine {
    Eof,
    /// A line with no newline yet. The writer is still on it.
    Partial,
    Complete {
        text: String,
        read: u64,
    },
    /// Consumed and deliberately not returned. Carries the stable reason and how far to advance.
    Skipped {
        reason: &'static str,
        read: u64,
    },
}

/// Reads one line, never allocating more than the ceiling.
///
/// `read_line` would allocate whatever the line happens to be and fail the whole batch on invalid
/// UTF-8, which turns one damaged byte into a file that can never be indexed. Here both are
/// bounded, local, and reported as a reason code — and in neither case does a byte of the offending
/// line reach a diagnostic, because those bytes are exactly the content nobody managed to redact.
fn read_bounded_line(reader: &mut BufReader<File>) -> Result<BoundedLine, OperationsLogError> {
    let mut raw = Vec::new();
    let read = std::io::Read::take(std::io::Read::by_ref(reader), MAX_LOG_LINE_BYTES)
        .read_until(b'\n', &mut raw)
        .map_err(|_| OperationsLogError::RepairFailed("log_source_read_failed"))?
        as u64;
    if read == 0 {
        return Ok(BoundedLine::Eof);
    }
    if !raw.ends_with(b"\n") {
        if read < MAX_LOG_LINE_BYTES {
            return Ok(BoundedLine::Partial);
        }
        // The ceiling was reached with no newline. Walk forward to the next one in fixed chunks so
        // the skip costs a constant amount of memory however long the line turns out to be.
        drop(raw);
        let skipped = skip_to_newline(reader)?;
        return Ok(match skipped {
            // No newline before the end of the file: the rest is one unterminated line, and it is
            // still being written or was truncated mid-write. Leaving it costs nothing.
            None => BoundedLine::Partial,
            Some(extra) => BoundedLine::Skipped {
                reason: "log_line_too_large",
                read: read + extra,
            },
        });
    }
    match String::from_utf8(raw) {
        Ok(text) => Ok(BoundedLine::Complete { text, read }),
        // The line is complete and its bytes are not text. Advancing past it is what keeps one
        // damaged region from stalling the file forever.
        Err(_) => Ok(BoundedLine::Skipped {
            reason: "log_line_invalid_utf8",
            read,
        }),
    }
}

/// Discards up to the next newline in fixed chunks, returning how many bytes went.
fn skip_to_newline(reader: &mut BufReader<File>) -> Result<Option<u64>, OperationsLogError> {
    let mut discarded = 0u64;
    let mut chunk = Vec::with_capacity(SKIP_CHUNK_BYTES);
    loop {
        chunk.clear();
        let read = std::io::Read::take(std::io::Read::by_ref(reader), SKIP_CHUNK_BYTES as u64)
            .read_until(b'\n', &mut chunk)
            .map_err(|_| OperationsLogError::RepairFailed("log_source_read_failed"))?
            as u64;
        if read == 0 {
            return Ok(None);
        }
        discarded += read;
        if chunk.ends_with(b"\n") {
            return Ok(Some(discarded));
        }
    }
}

impl RedactedLogSourceReader for UnifiedLogSourceReader {
    /// The retained corpus as it stands right now, with each file's length captured alongside it.
    ///
    /// The length is read once, here, rather than per batch. A pass that re-read it would chase a
    /// file that is still being appended to and never declare itself caught up; capturing it makes
    /// "caught up" mean "reached the end that existed when I started", which is a claim that can
    /// actually become true.
    fn sources(&self) -> Result<Vec<LogSourceSnapshot>, OperationsLogError> {
        let directory = std::fs::read_dir(&self.log_dir);
        // A directory that cannot be listed is not an empty directory. Reporting it as one would
        // let retention conclude that every source expired and delete the whole index.
        if let Err(error) = directory {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(OperationsLogError::RepairFailed("log_directory_unreadable"));
            }
        }
        Ok(self
            .files()
            .iter()
            .map(|path| LogSourceSnapshot {
                identity: self.identity(path),
                end_offset: std::fs::metadata(path).map(|data| data.len()).unwrap_or(0),
            })
            .collect())
    }

    /// Reads complete lines from an offset, bounded three ways.
    ///
    /// A trailing line without its newline is left alone and the offset is not advanced past it:
    /// the writer has not finished it, and consuming it would index half a record and never see
    /// the other half.
    fn read_batch(
        &self,
        source: &LogSourceIdentity,
        from_offset: u64,
        max_records: usize,
        max_bytes: u64,
    ) -> Result<RedactedLogBatch, OperationsLogError> {
        let Some(path) = self.path_for(source) else {
            // The file is gone. That is retention doing its job, not a failure: the batch reports
            // the end so the caller stops rather than retrying a path that will never return.
            return Ok(RedactedLogBatch {
                next_offset: from_offset,
                reached_end: true,
                ..RedactedLogBatch::default()
            });
        };
        let mut file = File::open(&path)
            .map_err(|_| OperationsLogError::RepairFailed("log_source_unreadable"))?;
        file.seek(SeekFrom::Start(from_offset))
            .map_err(|_| OperationsLogError::RepairFailed("log_source_seek_failed"))?;
        let mut reader = BufReader::new(file);

        let mut records = Vec::new();
        let mut rejections = LineRejections::new();
        let mut offset = from_offset;
        let mut consumed = 0u64;
        let mut reached_end = false;
        loop {
            if records.len() >= max_records || consumed >= max_bytes {
                break;
            }
            match read_bounded_line(&mut reader)? {
                BoundedLine::Eof => {
                    reached_end = true;
                    break;
                }
                // A partial trailing line. Stop without advancing: the writer has not finished it,
                // and the next pass will see it whole.
                BoundedLine::Partial => {
                    reached_end = true;
                    break;
                }
                BoundedLine::Complete { text, read } => {
                    let trimmed = text.trim_end_matches(['\n', '\r']);
                    match to_record(source, offset, offset + read, trimmed) {
                        Some(record) => records.push(record),
                        // Complete but unusable. Counted so coverage can say a record is missing,
                        // and the offset still advances so one bad line cannot stall the file.
                        None if !trimmed.is_empty() => {
                            *rejections.entry("log_record_rejected").or_default() += 1;
                        }
                        None => {}
                    }
                    offset += read;
                    consumed += read;
                }
                // Skipped without ever holding the whole line: a single pathological line must not
                // decide how much memory the indexer uses.
                BoundedLine::Skipped { reason, read } => {
                    *rejections.entry(reason).or_default() += 1;
                    offset += read;
                    consumed += read;
                }
            }
        }
        Ok(RedactedLogBatch {
            records,
            next_offset: offset,
            rejections,
            reached_end,
        })
    }

    /// The file names an export would read, never their paths.
    ///
    /// A name says which log; a path says where the user keeps their data. The caller only needs to
    /// tell the user what an export covers, and the export itself resolves the names against the
    /// configured directory rather than against something a client sent back.
    fn export_sources(&self) -> Result<Vec<String>, OperationsLogError> {
        Ok(self
            .files()
            .iter()
            .filter_map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::io::Write;

    fn write_lines(path: &Path, lines: &[&str]) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open log");
        for line in lines {
            writeln!(file, "{line}").expect("write line");
        }
    }

    fn entry(message: &str, record_id: Option<&str>) -> String {
        let id = record_id
            .map(|value| format!(r#","recordId":"{value}""#))
            .unwrap_or_default();
        format!(
            r#"{{"timestamp":"2026-08-24T10:00:00Z","level":"info","category":"test","message":"{message}","context":{{"sessionId":"session-1"}}{id}}}"#
        )
    }

    #[test]
    fn a_record_keeps_the_identity_its_line_carries() {
        let fixture = TempDirectory::new("log-source-identity");
        let path = fixture.path().join(LOG_FILE_NAME);
        write_lines(&path, &[&entry("hello", Some("record-7"))]);
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let source = reader.sources().expect("sources").remove(0);

        let batch = reader
            .read_batch(&source.identity, 0, 100, 1_000_000)
            .expect("batch");

        assert_eq!(batch.records[0].record_id, "record-7");
        assert_eq!(
            batch.records[0].correlation.session_id.as_deref(),
            Some("session-1")
        );
    }

    /// A repeated pass over the same historical line has to produce the same id, or every repair
    /// would add a second copy of every record written before ids existed.
    #[test]
    fn a_line_without_an_id_gets_the_same_derived_id_every_time() {
        let fixture = TempDirectory::new("log-source-legacy");
        let path = fixture.path().join(LOG_FILE_NAME);
        write_lines(&path, &[&entry("hello", None), &entry("hello", None)]);
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let source = reader.sources().expect("sources").remove(0);

        let first = reader
            .read_batch(&source.identity, 0, 100, 1_000_000)
            .expect("first");
        let again = reader
            .read_batch(&source.identity, 0, 100, 1_000_000)
            .expect("again");

        assert_eq!(first.records[0].record_id, again.records[0].record_id);
        // Two identical lines at different offsets are two records, not one: a retry loop logging
        // the same failure twice really did log twice.
        assert_ne!(first.records[0].record_id, first.records[1].record_id);
        assert!(first.records[0].record_id.starts_with("legacy:v1:"));
    }

    #[test]
    fn a_partial_trailing_line_does_not_advance_the_checkpoint() {
        let fixture = TempDirectory::new("log-source-partial");
        let path = fixture.path().join(LOG_FILE_NAME);
        write_lines(&path, &[&entry("complete", Some("record-1"))]);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append");
        write!(file, "{}", entry("half-written", Some("record-2"))).expect("partial");
        drop(file);
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let source = reader.sources().expect("sources").remove(0);

        let batch = reader
            .read_batch(&source.identity, 0, 100, 1_000_000)
            .expect("batch");

        assert_eq!(batch.records.len(), 1);
        assert!(batch.reached_end);
        // The offset stops at the end of the complete line, so the next pass sees the rest whole.
        // `writeln!` emits a bare newline on every platform, so a complete line is its bytes + 1.
        assert_eq!(
            batch.next_offset as usize,
            entry("complete", Some("record-1")).len() + 1
        );
    }

    /// One unusable line must not stall the file behind it. It is counted and stepped over.
    #[test]
    fn a_malformed_complete_line_is_counted_and_stepped_over() {
        let fixture = TempDirectory::new("log-source-malformed");
        let path = fixture.path().join(LOG_FILE_NAME);
        write_lines(
            &path,
            &["{not json at all", &entry("after", Some("record-2"))],
        );
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let source = reader.sources().expect("sources").remove(0);

        let batch = reader
            .read_batch(&source.identity, 0, 100, 1_000_000)
            .expect("batch");

        assert_eq!(batch.rejected_total(), 1);
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.records[0].record_id, "record-2");
    }

    #[test]
    fn a_batch_stops_at_its_record_bound_without_losing_its_place() {
        let fixture = TempDirectory::new("log-source-bounded");
        let path = fixture.path().join(LOG_FILE_NAME);
        let lines: Vec<String> = (0..5)
            .map(|index| entry("line", Some(&format!("record-{index}"))))
            .collect();
        write_lines(&path, &lines.iter().map(String::as_str).collect::<Vec<_>>());
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let source = reader.sources().expect("sources").remove(0);

        let first = reader
            .read_batch(&source.identity, 0, 2, 1_000_000)
            .expect("first");
        let second = reader
            .read_batch(&source.identity, first.next_offset, 2, 1_000_000)
            .expect("second");

        assert_eq!(first.records.len(), 2);
        assert!(!first.reached_end);
        assert_eq!(second.records[0].record_id, "record-2");
    }

    /// A source that retention removed is an end, not an error: retrying a path that will never
    /// come back would keep a repair looping over nothing.
    #[test]
    fn a_removed_source_reports_the_end_rather_than_failing() {
        let fixture = TempDirectory::new("log-source-missing");
        let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
        let missing = LogSourceIdentity {
            directory_generation: "gone".to_string(),
            file_id: "gone".to_string(),
        };

        let batch = reader.read_batch(&missing, 0, 10, 1_000).expect("batch");

        assert!(batch.reached_end);
        assert!(batch.records.is_empty());
    }
}
