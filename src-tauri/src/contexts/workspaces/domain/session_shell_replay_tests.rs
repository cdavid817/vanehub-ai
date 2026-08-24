use super::session_shell::ShellStream;
use super::session_shell_replay::{ShellReplayBuffer, MAX_SHELL_REPLAY_BYTES};

fn buffer() -> ShellReplayBuffer {
    ShellReplayBuffer::default()
}

fn push(buffer: &mut ShellReplayBuffer, bytes: &[u8]) -> Option<String> {
    buffer
        .push_bytes(ShellStream::Pty, "2026-08-24T10:00:00Z", bytes)
        .map(|frame| frame.data)
}

#[test]
fn sequences_start_at_one_and_never_repeat() {
    let mut buffer = buffer();
    for index in 1..=3u64 {
        let frame = buffer
            .push_bytes(ShellStream::Pty, "2026-08-24T10:00:00Z", b"x")
            .expect("frame");
        assert_eq!(frame.sequence, index);
    }
    assert_eq!(buffer.next_sequence(), 4);
}

/// A PTY returns whatever bytes were available, so a multi-byte character routinely straddles two
/// reads. Emitting the first half would put a replacement character in the middle of ordinary
/// non-ASCII output — text the program never wrote.
#[test]
fn a_code_point_split_across_two_reads_emits_once_and_whole() {
    let mut buffer = buffer();
    let bytes = "工作区".as_bytes();

    assert_eq!(push(&mut buffer, &bytes[..4]), Some("工".to_string()));
    // The remaining two bytes of the second character alone are not a character.
    assert_eq!(push(&mut buffer, &bytes[4..5]), None);
    assert_eq!(push(&mut buffer, &bytes[5..]), Some("作区".to_string()));
}

/// One bad byte must not stall everything behind it. The stream keeps moving with a replacement
/// character, which is what a terminal would show anyway.
#[test]
fn an_invalid_byte_does_not_stall_the_stream() {
    let mut buffer = buffer();
    let emitted = push(&mut buffer, &[0x41, 0xff, 0x42]).expect("frame");
    assert!(emitted.starts_with('A'), "{emitted}");
    assert!(emitted.ends_with('B'), "{emitted}");
}

#[test]
fn a_snapshot_returns_everything_after_the_consumed_sequence() {
    let mut buffer = buffer();
    for text in ["one", "two", "three"] {
        push(&mut buffer, text.as_bytes());
    }

    let snapshot = buffer.snapshot(1);

    assert_eq!(
        snapshot
            .frames
            .iter()
            .map(|frame| frame.data.clone())
            .collect::<Vec<_>>(),
        vec!["two".to_string(), "three".to_string()]
    );
    assert!(snapshot.gap.is_none());
    // Exact, so a subscriber can tell a gap from a race by comparing rather than by guessing.
    assert_eq!(snapshot.next_sequence, 4);
}

/// Whole frames only. Trimming inside a frame would trim inside a byte string that may hold half an
/// escape sequence, and a terminal handed half an escape sequence renders the rest of it as text.
#[test]
fn output_past_the_bound_evicts_whole_frames_and_reports_one_gap() {
    let mut buffer = buffer();
    let chunk = "a".repeat(64 * 1024);
    for _ in 0..20 {
        push(&mut buffer, chunk.as_bytes());
    }

    assert!(
        buffer.retained_bytes() <= MAX_SHELL_REPLAY_BYTES,
        "retained {} bytes",
        buffer.retained_bytes()
    );
    let snapshot = buffer.snapshot(0);
    let gap = snapshot.gap.expect("an evicted buffer reports its gap");
    assert_eq!(gap.from_sequence, 1);
    // Contiguous and singular: eviction only removes from the oldest end, so what remains is one
    // unbroken run and there is nothing for a second gap to sit between.
    assert_eq!(gap.to_sequence + 1, snapshot.frames[0].sequence);
    for pair in snapshot.frames.windows(2) {
        assert_eq!(pair[1].sequence, pair[0].sequence + 1);
    }
}

#[test]
fn a_snapshot_taken_beyond_the_newest_frame_is_empty_without_a_gap() {
    let mut buffer = buffer();
    push(&mut buffer, b"one");

    let snapshot = buffer.snapshot(1);

    assert!(snapshot.frames.is_empty());
    assert!(snapshot.gap.is_none());
    assert_eq!(snapshot.next_sequence, 2);
}

/// Releasing frees the text without disturbing the counter, so a closed Shell stops holding a
/// megabyte while anything still reading its descriptor keeps a consistent view of the sequence.
#[test]
fn releasing_frees_bytes_without_rewinding_the_sequence() {
    let mut buffer = buffer();
    push(&mut buffer, b"one");
    let next = buffer.next_sequence();

    buffer.release();

    assert_eq!(buffer.retained_bytes(), 0);
    assert_eq!(buffer.next_sequence(), next);
    assert!(buffer.snapshot(0).gap.is_some());
}
