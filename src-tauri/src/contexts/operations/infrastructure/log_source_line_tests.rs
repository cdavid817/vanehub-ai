//! The four things a line can be that is not a record.
//!
//! Each gets a different answer, and the differences are the whole point. A partial line is one the
//! writer has not finished, so it must be left exactly where it is. The other three are complete and
//! unusable, so the offset must advance past them — otherwise one bad line stalls the file forever
//! and every record after it becomes permanently invisible.
//!
//! Nothing here ever quotes the offending bytes. Those bytes are precisely the content nobody
//! managed to redact, and a diagnostic that echoed them would put unredacted text into a second
//! place redaction never runs.

use super::log_source_reader::{UnifiedLogSourceReader, MAX_LOG_LINE_BYTES};
use crate::contexts::operations::application::RedactedLogSourceReader;
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;
use std::io::Write;
use std::path::Path;

fn entry(message: &str, record_id: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-08-25T10:00:00Z","level":"info","category":"test","message":"{message}","context":{{}},"recordId":"{record_id}"}}"#
    )
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open log");
    file.write_all(bytes).expect("write");
}

struct Fixture {
    _directory: TempDirectory,
    reader: UnifiedLogSourceReader,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    Fixture {
        reader: UnifiedLogSourceReader::new(directory.path().to_path_buf()),
        _directory: directory,
    }
}

fn log_path(fixture: &Fixture) -> std::path::PathBuf {
    fixture._directory.path().join(LOG_FILE_NAME)
}

/// A trailing line without its newline is left alone entirely.
///
/// No row, no gap, and — the part that matters — no advance. The writer is mid-append; consuming it
/// would index half a record and then never see the other half, because the next pass would resume
/// after bytes it already read.
#[test]
fn a_partial_trailing_line_is_neither_indexed_nor_counted_as_a_gap() {
    let fixture = fixture("log-line-partial");
    let path = log_path(&fixture);
    write_bytes(
        &path,
        format!("{}\n", entry("complete", "record-1")).as_bytes(),
    );
    let complete_end = std::fs::metadata(&path).expect("metadata").len();
    // The line the writer has not finished.
    write_bytes(&path, entry("half written", "record-2").as_bytes());

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 1_000_000)
        .expect("batch");

    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.records[0].record_id, "record-1");
    assert!(
        batch.rejections.is_empty(),
        "an unfinished line was reported as a gap: {:?}",
        batch.rejections
    );
    assert_eq!(
        batch.next_offset, complete_end,
        "the offset advanced into a line the writer had not finished"
    );
    assert!(batch.reached_end);
}

/// The same partial line, once finished, is indexed on the next pass.
///
/// This is what makes stopping correct rather than merely cautious: nothing is lost by waiting.
///
/// The source is re-listed for the second read, exactly as a pass does. It has to be: a file whose
/// very first line is still being written has no complete head line, so its identity is the
/// filesystem half alone, and completing that line gives it the content half too. Nothing is
/// stranded by that — a partial line never advances an offset, so there is no checkpoint under the
/// earlier identity to lose.
#[test]
fn a_partial_line_is_indexed_once_the_writer_finishes_it() {
    let fixture = fixture("log-line-partial-completed");
    let path = log_path(&fixture);
    write_bytes(&path, entry("in progress", "record-1").as_bytes());
    let first_source = fixture.reader.sources().expect("sources").remove(0);
    let first = fixture
        .reader
        .read_batch(&first_source.identity, 0, 100, 1_000_000)
        .expect("first");

    write_bytes(&path, b"\n");
    let second_source = fixture.reader.sources().expect("sources").remove(0);
    let second = fixture
        .reader
        .read_batch(&second_source.identity, first.next_offset, 100, 1_000_000)
        .expect("second");

    assert!(first.records.is_empty());
    assert_eq!(first.next_offset, 0);
    assert_eq!(second.records.len(), 1);
    assert_eq!(second.records[0].record_id, "record-1");
}

/// A file whose first line is already complete keeps one identity as it grows.
///
/// The companion to the case above, and the one that actually matters for checkpoints: every file
/// that has ever been read has a complete first line, so its identity is settled from then on and
/// an append never strands the offset a pass committed.
#[test]
fn a_file_with_a_complete_head_line_keeps_its_identity_as_it_grows() {
    let fixture = fixture("log-line-stable-identity");
    let path = log_path(&fixture);
    write_bytes(
        &path,
        format!("{}\n", entry("first", "record-1")).as_bytes(),
    );
    let before = fixture.reader.sources().expect("sources").remove(0);

    write_bytes(
        &path,
        format!("{}\n", entry("second", "record-2")).as_bytes(),
    );
    let after = fixture.reader.sources().expect("sources").remove(0);

    assert_eq!(before.identity, after.identity);
    assert!(
        after.end_offset > before.end_offset,
        "the captured target did not grow with the file"
    );
}

/// A complete line that is not a record becomes a gap and the offset moves past it.
///
/// Both halves matter. The gap is how coverage stops claiming to be whole; the advance is how one
/// producer bug stops costing every record written after it.
#[test]
fn a_malformed_complete_line_becomes_a_gap_and_the_offset_advances_past_it() {
    let fixture = fixture("log-line-malformed");
    let path = log_path(&fixture);
    write_bytes(&path, b"{this is not json at all\n");
    write_bytes(
        &path,
        format!("{}\n", entry("after", "record-1")).as_bytes(),
    );

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 1_000_000)
        .expect("batch");

    assert_eq!(batch.rejections.get("log_record_rejected"), Some(&1));
    // The record after the bad line was still reached, which is the advance doing its job.
    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.records[0].record_id, "record-1");
}

/// A line past the ceiling is skipped to the next newline without ever being held.
///
/// The ceiling is not about typical lines. It is about the one that lost its newline to a partial
/// write, in which case "the line" is the rest of the file — and without a bound the indexer's
/// memory is decided by the worst line anyone ever wrote.
#[test]
fn an_oversized_line_is_skipped_to_the_newline_and_reported_as_too_large() {
    let fixture = fixture("log-line-oversized");
    let path = log_path(&fixture);
    // Comfortably past the ceiling, and still a single line.
    let huge = vec![b'x'; (MAX_LOG_LINE_BYTES + 4096) as usize];
    write_bytes(&path, &huge);
    write_bytes(&path, b"\n");
    write_bytes(
        &path,
        format!("{}\n", entry("after", "record-1")).as_bytes(),
    );

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 100 * 1024 * 1024)
        .expect("batch");

    assert_eq!(batch.rejections.get("log_line_too_large"), Some(&1));
    // The reader resumed on the record after it rather than stalling, and the offset it advanced by
    // covers the whole oversized line plus its newline.
    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.records[0].record_id, "record-1");
    assert_eq!(
        batch.records[0].source_offset,
        MAX_LOG_LINE_BYTES + 4096 + 1
    );
}

/// An oversized line with no newline after it is treated as unfinished, not as a gap.
///
/// It may still be growing. Reporting a gap for a line that has not ended yet would record a loss
/// that has not happened, and coverage would never recover from it.
#[test]
fn an_oversized_line_with_no_newline_yet_is_left_for_the_next_pass() {
    let fixture = fixture("log-line-oversized-unterminated");
    let path = log_path(&fixture);
    write_bytes(&path, &vec![b'x'; (MAX_LOG_LINE_BYTES + 512) as usize]);

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 100 * 1024 * 1024)
        .expect("batch");

    assert!(batch.records.is_empty());
    assert!(
        batch.rejections.is_empty(),
        "a line that has not ended was counted as lost: {:?}",
        batch.rejections
    );
    assert_eq!(batch.next_offset, 0, "the offset advanced past a live line");
}

/// Bytes that are not text are skipped with a code, and the code is all anyone sees.
///
/// The bytes themselves never reach a diagnostic, a log, or an event: they are the one thing in the
/// corpus that provably has not been through redaction, because nothing could parse them.
#[test]
fn an_invalid_utf8_line_is_skipped_with_a_reason_code_and_never_quoted() {
    let fixture = fixture("log-line-invalid-utf8");
    let path = log_path(&fixture);
    // A lone continuation byte: valid as a byte, not valid as text.
    write_bytes(&path, &[0x7b, 0x22, 0xff, 0xfe, 0x22, 0x7d, b'\n']);
    write_bytes(
        &path,
        format!("{}\n", entry("after", "record-1")).as_bytes(),
    );

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 1_000_000)
        .expect("batch");

    assert_eq!(batch.rejections.get("log_line_invalid_utf8"), Some(&1));
    assert_eq!(batch.records.len(), 1);
    // Every reason is a stable code with no payload, so there is nowhere for a byte to travel.
    for reason in batch.rejections.keys() {
        assert!(
            reason.is_ascii() && !reason.contains(|character: char| character.is_ascii_control()),
            "{reason:?} is not a plain code"
        );
    }
}

/// The three reasons are counted separately.
///
/// A single total would present a producer bug, a bound doing its job, and a damaged file as one
/// condition — and the response to each is different.
#[test]
fn each_kind_of_unusable_line_is_counted_under_its_own_reason() {
    let fixture = fixture("log-line-mixed");
    let path = log_path(&fixture);
    write_bytes(&path, b"{not json\n");
    write_bytes(&path, &[0x7b, 0xff, 0x7d, b'\n']);
    write_bytes(&path, &vec![b'x'; (MAX_LOG_LINE_BYTES + 16) as usize]);
    write_bytes(&path, b"\n");
    write_bytes(
        &path,
        format!("{}\n", entry("survivor", "record-1")).as_bytes(),
    );

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 100, 100 * 1024 * 1024)
        .expect("batch");

    assert_eq!(batch.rejections.get("log_record_rejected"), Some(&1));
    assert_eq!(batch.rejections.get("log_line_invalid_utf8"), Some(&1));
    assert_eq!(batch.rejections.get("log_line_too_large"), Some(&1));
    assert_eq!(batch.rejected_total(), 3);
    assert_eq!(
        batch.records.len(),
        1,
        "the good record after them was lost"
    );
}

/// A batch stops at the record bound and reports where to resume.
#[test]
fn a_batch_stops_at_the_record_bound_and_names_its_resume_point() {
    let fixture = fixture("log-line-record-bound");
    let path = log_path(&fixture);
    for index in 0..10 {
        write_bytes(
            &path,
            format!("{}\n", entry("line", &format!("record-{index}"))).as_bytes(),
        );
    }

    let source = fixture.reader.sources().expect("sources").remove(0);
    let first = fixture
        .reader
        .read_batch(&source.identity, 0, 4, 1_000_000)
        .expect("first");
    let second = fixture
        .reader
        .read_batch(&source.identity, first.next_offset, 4, 1_000_000)
        .expect("second");

    assert_eq!(first.records.len(), 4);
    assert!(!first.reached_end);
    assert_eq!(second.records.len(), 4);
    // Contiguous: the second batch starts on the record after the first batch's last.
    assert_eq!(second.records[0].record_id, "record-4");
}

/// A batch stops at the byte bound too, so a file of few enormous records cannot be read whole.
#[test]
fn a_batch_stops_at_the_byte_bound() {
    let fixture = fixture("log-line-byte-bound");
    let path = log_path(&fixture);
    for index in 0..10 {
        write_bytes(
            &path,
            format!("{}\n", entry(&"p".repeat(400), &format!("record-{index}"))).as_bytes(),
        );
    }

    let source = fixture.reader.sources().expect("sources").remove(0);
    let batch = fixture
        .reader
        .read_batch(&source.identity, 0, 1_000, 1_000)
        .expect("batch");

    assert!(
        batch.records.len() < 10,
        "the byte bound did not stop the batch"
    );
    assert!(!batch.reached_end);
}
