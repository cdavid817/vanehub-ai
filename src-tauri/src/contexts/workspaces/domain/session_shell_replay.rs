//! Bounded, UTF-8-safe replay for one retained Session Shell.

use super::session_shell::{ShellOutputFrame, ShellReasonCode, ShellReplayGap, ShellStream};
use std::collections::VecDeque;

/// How much output one Shell keeps for a view that comes back.
///
/// Bytes rather than frames, because a frame is whatever size a PTY read happened to return: a
/// frame-counted bound would hold a megabyte of one build's output and a kilobyte of another's, and
/// only one of those fits in memory a hundred shells at a time.
pub(crate) const MAX_SHELL_REPLAY_BYTES: usize = 1024 * 1024;

/// The longest incomplete UTF-8 sequence a decoder can legitimately be waiting on.
const MAX_PENDING_BYTES: usize = 3;

/// What an attaching view is given.
pub(crate) struct ShellReplaySnapshot {
    pub(crate) frames: Vec<ShellOutputFrame>,
    pub(crate) gap: Option<ShellReplayGap>,
    /// The sequence the next frame will carry. Exact, so a subscriber can tell a gap from a race
    /// by comparing rather than by inferring one from timing.
    pub(crate) next_sequence: u64,
}

/// Retained output for one Shell.
///
/// Sequences start at 1 so that "nothing consumed" can be said with 0 without an `Option` that
/// every caller would have to unwrap the same way.
pub(crate) struct ShellReplayBuffer {
    frames: VecDeque<ShellOutputFrame>,
    bytes: usize,
    next_sequence: u64,
    /// Bytes of a code point split across two reads, carried into the next one.
    pending: Vec<u8>,
    evicted: bool,
}

impl Default for ShellReplayBuffer {
    fn default() -> Self {
        Self {
            frames: VecDeque::new(),
            bytes: 0,
            next_sequence: 1,
            pending: Vec::new(),
            evicted: false,
        }
    }
}

impl ShellReplayBuffer {
    /// Decodes one read and retains whatever complete text it produced.
    ///
    /// Returns `None` when the read contained only the first half of a code point. A PTY hands back
    /// whatever bytes were available, so a multi-byte character routinely straddles two reads;
    /// emitting the first half would put a replacement character into the terminal that no program
    /// ever wrote, and it would do it in the middle of ordinary non-ASCII output.
    pub(crate) fn push_bytes(
        &mut self,
        stream: ShellStream,
        occurred_at: &str,
        chunk: &[u8],
    ) -> Option<ShellOutputFrame> {
        let text = self.decode(chunk);
        if text.is_empty() {
            return None;
        }
        let frame = ShellOutputFrame {
            sequence: self.next_sequence,
            occurred_at: occurred_at.to_string(),
            stream,
            data: text,
        };
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.bytes = self.bytes.saturating_add(frame.data.len());
        self.frames.push_back(frame.clone());
        self.evict_to_bound();
        Some(frame)
    }

    /// Whole frames only, oldest first.
    ///
    /// Trimming inside a frame would be trimming inside a byte string that may hold a partial
    /// escape sequence, and a terminal handed half an escape sequence renders the rest of it as
    /// text.
    fn evict_to_bound(&mut self) {
        while self.bytes > MAX_SHELL_REPLAY_BYTES && self.frames.len() > 1 {
            if let Some(dropped) = self.frames.pop_front() {
                self.bytes = self.bytes.saturating_sub(dropped.data.len());
                self.evicted = true;
            }
        }
    }

    fn decode(&mut self, chunk: &[u8]) -> String {
        self.pending.extend_from_slice(chunk);
        let mut text = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(valid) => {
                    text.push_str(valid);
                    self.pending.clear();
                    break;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    if let Ok(valid) = std::str::from_utf8(&self.pending[..valid_up_to]) {
                        text.push_str(valid);
                    }
                    match error.error_len() {
                        // Genuinely invalid bytes. Replace and continue, so one bad byte cannot
                        // stall the stream behind it forever.
                        Some(length) => {
                            text.push('\u{FFFD}');
                            self.pending.drain(..valid_up_to + length);
                        }
                        // A split code point: keep the suffix for the next read.
                        None => {
                            self.pending.drain(..valid_up_to);
                            break;
                        }
                    }
                }
            }
        }
        // A pending suffix is at most three bytes by definition; anything longer means the decoder
        // is holding something it will never complete.
        if self.pending.len() > MAX_PENDING_BYTES {
            self.pending.clear();
        }
        text
    }

    /// Everything after `after`, plus one gap when the buffer no longer reaches back that far.
    ///
    /// `after` is the last sequence the caller consumed; 0 means it has consumed nothing.
    pub(crate) fn snapshot(&self, after: u64) -> ShellReplaySnapshot {
        let start = after.saturating_add(1);
        let retained_floor = self
            .frames
            .front()
            .map(|frame| frame.sequence)
            .unwrap_or(self.next_sequence);
        let gap = if retained_floor > start {
            // Only ever one, and only ever at the front: eviction removes from the oldest end, so
            // what remains is contiguous.
            Some(ShellReplayGap {
                from_sequence: start,
                to_sequence: retained_floor - 1,
                reason: shell_reason(if self.evicted {
                    "shell_replay_evicted"
                } else {
                    "shell_replay_unavailable"
                }),
            })
        } else {
            None
        };
        ShellReplaySnapshot {
            frames: self
                .frames
                .iter()
                .filter(|frame| frame.sequence >= start)
                .cloned()
                .collect(),
            gap,
            next_sequence: self.next_sequence,
        }
    }

    pub(crate) fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.bytes
    }

    /// Frees the retained text without disturbing the sequence, so a closed Shell stops holding a
    /// megabyte while its descriptor is still being read.
    pub(crate) fn release(&mut self) {
        self.frames.clear();
        self.pending.clear();
        self.bytes = 0;
        self.evicted = true;
    }
}

/// A reason code built without a fallible parse at every call site.
pub(crate) fn shell_reason(code: &str) -> ShellReasonCode {
    ShellReasonCode::sanitized(code)
}
