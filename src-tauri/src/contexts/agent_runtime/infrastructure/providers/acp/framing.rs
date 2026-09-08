//! Bounded newline-delimited JSON framing for ACP stdio.
//!
//! The transport is UTF-8 JSON-RPC, one message per `\n`, no `Content-Length` header, nothing on
//! stdout that is not a message. The decoder accepts bytes in any partition -- a multibyte
//! character split across two reads is the ordinary case, not an edge case -- and refuses a frame
//! that exceeds the byte budget or the nesting budget with a typed reason. Refusal is final for
//! the connection: unlike the headless framer, an oversized ACP frame cannot be skipped, because
//! the frame after it may be the response to a request the host is waiting on and the skipped one
//! may have been the approval that authorized it.

use super::budget::{MAX_FRAME_BYTES, MAX_JSON_DEPTH};
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FramingError {
    /// A frame exceeded the byte budget before its newline arrived.
    FrameTooLarge { limit: usize },
    /// A completed frame was not valid UTF-8.
    InvalidUtf8,
    /// A completed frame was not a JSON document. The agent wrote something that is not ACP to
    /// its stdout -- typically a banner.
    NotJson { preview: String },
    /// A completed frame nests deeper than the budget allows.
    TooDeep { limit: usize },
}

impl FramingError {
    pub(crate) fn reason_code(&self) -> &'static str {
        match self {
            Self::FrameTooLarge { .. } => "acp-frame-too-large",
            Self::InvalidUtf8 => "acp-frame-invalid-utf8",
            Self::NotJson { .. } => "acp-stdout-not-protocol",
            Self::TooDeep { .. } => "acp-frame-too-deep",
        }
    }
}

impl fmt::Display for FramingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLarge { limit } => {
                write!(formatter, "ACP frame exceeded {limit} bytes")
            }
            Self::InvalidUtf8 => formatter.write_str("ACP frame is not valid UTF-8"),
            Self::NotJson { preview } => {
                write!(formatter, "ACP stdout carried non-protocol data: {preview}")
            }
            Self::TooDeep { limit } => {
                write!(formatter, "ACP frame nests deeper than {limit} levels")
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct NdjsonDecoder {
    buffer: Vec<u8>,
    max_frame_bytes: usize,
    max_depth: usize,
    failed: bool,
}

impl Default for NdjsonDecoder {
    fn default() -> Self {
        Self::new(MAX_FRAME_BYTES, MAX_JSON_DEPTH)
    }
}

impl NdjsonDecoder {
    pub(crate) fn new(max_frame_bytes: usize, max_depth: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_bytes,
            max_depth,
            failed: false,
        }
    }

    /// Feeds a chunk and returns every complete frame it closed, in order. After an error the
    /// decoder stays failed: later bytes are discarded rather than resynchronized on.
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<Vec<Value>, FramingError> {
        if self.failed {
            return Ok(Vec::new());
        }
        let mut frames = Vec::new();
        let mut start = 0;
        for (index, byte) in chunk.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            let segment = &chunk[start..index];
            if self.buffer.len() + segment.len() > self.max_frame_bytes {
                self.fail();
                return Err(FramingError::FrameTooLarge {
                    limit: self.max_frame_bytes,
                });
            }
            let mut line = std::mem::take(&mut self.buffer);
            line.extend_from_slice(segment);
            start = index + 1;
            // A carriage return before the newline is tolerated; an all-whitespace line is not a
            // message and is skipped rather than failed, which is what the NDJSON convention does.
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match self.decode_line(line) {
                Ok(value) => frames.push(value),
                Err(error) => {
                    self.fail();
                    return Err(error);
                }
            }
        }
        let tail = &chunk[start..];
        if self.buffer.len() + tail.len() > self.max_frame_bytes {
            self.fail();
            return Err(FramingError::FrameTooLarge {
                limit: self.max_frame_bytes,
            });
        }
        self.buffer.extend_from_slice(tail);
        Ok(frames)
    }

    /// Bytes held for an unterminated frame. Non-empty at EOF means the agent died mid-message.
    pub(crate) fn pending_bytes(&self) -> usize {
        self.buffer.len()
    }

    fn fail(&mut self) {
        self.failed = true;
        self.buffer.clear();
    }

    fn decode_line(&self, line: Vec<u8>) -> Result<Value, FramingError> {
        let text = String::from_utf8(line).map_err(|_| FramingError::InvalidUtf8)?;
        let value: Value = serde_json::from_str(&text).map_err(|_| FramingError::NotJson {
            preview: safe_preview(&text),
        })?;
        if depth_of(&value) > self.max_depth {
            return Err(FramingError::TooDeep {
                limit: self.max_depth,
            });
        }
        Ok(value)
    }
}

/// The first few characters of a rejected line, redacted and bounded, for a diagnostic that says
/// "this was a banner" without echoing a page of output or a token.
fn safe_preview(text: &str) -> String {
    let redacted = crate::platform::logging::redact_text(text);
    let mut preview: String = redacted.chars().take(48).collect();
    if redacted.chars().count() > 48 {
        preview.push('…');
    }
    preview
}

fn depth_of(value: &Value) -> usize {
    // Iterative rather than recursive so a hostile frame cannot overflow the stack before the
    // budget is applied.
    let mut max_depth = 1;
    let mut stack: Vec<(&Value, usize)> = vec![(value, 1)];
    while let Some((current, depth)) = stack.pop() {
        max_depth = max_depth.max(depth);
        match current {
            Value::Array(items) => stack.extend(items.iter().map(|item| (item, depth + 1))),
            Value::Object(fields) => stack.extend(fields.values().map(|item| (item, depth + 1))),
            _ => {}
        }
    }
    max_depth
}

/// Encodes one outbound document as a single frame. A document that serializes with an embedded
/// newline cannot happen with `serde_json` (it escapes control characters), so the check is a
/// guard against a future encoder, not a live path.
pub(crate) fn encode_frame(document: &Value) -> Result<Vec<u8>, FramingError> {
    let mut bytes = serde_json::to_vec(document).map_err(|_| FramingError::NotJson {
        preview: "outbound document".to_string(),
    })?;
    if bytes.contains(&b'\n') {
        return Err(FramingError::NotJson {
            preview: "outbound document contains a newline".to_string(),
        });
    }
    if bytes.len() >= MAX_FRAME_BYTES {
        return Err(FramingError::FrameTooLarge {
            limit: MAX_FRAME_BYTES,
        });
    }
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frames_for(chunks: &[&[u8]]) -> Vec<Value> {
        let mut decoder = NdjsonDecoder::default();
        let mut frames = Vec::new();
        for chunk in chunks {
            frames.extend(decoder.push(chunk).expect("valid frames"));
        }
        assert_eq!(decoder.pending_bytes(), 0);
        frames
    }

    #[test]
    fn chinese_text_split_at_every_byte_boundary_decodes_identically() {
        let document = json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"会话","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"这是一段很长的中文回复，包含标点符号、emoji 🚀 和换行前的最后一个字"}}}});
        let encoded = encode_frame(&document).expect("encode");
        let whole = frames_for(&[&encoded]);
        for split in 1..encoded.len() {
            let (head, tail) = encoded.split_at(split);
            assert_eq!(frames_for(&[head, tail]), whole, "split at {split}");
        }
        let byte_by_byte: Vec<&[u8]> = encoded.chunks(1).collect();
        assert_eq!(frames_for(&byte_by_byte), whole);
        assert_eq!(whole[0], document);
    }

    #[test]
    fn several_frames_in_one_read_keep_their_order() {
        let first =
            encode_frame(&json!({"jsonrpc":"2.0","method":"a","params":{"n":1}})).expect("a");
        let second =
            encode_frame(&json!({"jsonrpc":"2.0","method":"b","params":{"n":2}})).expect("b");
        let third =
            encode_frame(&json!({"jsonrpc":"2.0","id":1,"result":{"stopReason":"end_turn"}}))
                .expect("c");
        let mut combined = first;
        combined.extend(b"\r\n");
        combined.extend(second);
        combined.extend(b"   \n");
        combined.extend(third);
        let frames = frames_for(&[&combined]);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0]["method"], json!("a"));
        assert_eq!(frames[1]["method"], json!("b"));
        assert_eq!(frames[2]["result"]["stopReason"], json!("end_turn"));
    }

    #[test]
    fn oversized_frames_fail_closed_and_stay_failed() {
        let mut decoder = NdjsonDecoder::new(64, MAX_JSON_DEPTH);
        let big = vec![b'x'; 65];
        let error = decoder.push(&big).expect_err("too large");
        assert_eq!(error, FramingError::FrameTooLarge { limit: 64 });
        assert_eq!(error.reason_code(), "acp-frame-too-large");
        // Nothing after a failure is decoded, even a valid frame.
        assert!(decoder
            .push(b"{\"jsonrpc\":\"2.0\",\"method\":\"x\"}\n")
            .expect("ignored")
            .is_empty());
        assert_eq!(decoder.pending_bytes(), 0);

        // The limit applies to the assembled frame, not each chunk.
        let mut decoder = NdjsonDecoder::new(64, MAX_JSON_DEPTH);
        assert!(decoder.push(&[b'y'; 40]).is_ok());
        assert!(matches!(
            decoder.push(&[b'y'; 40]),
            Err(FramingError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn banners_invalid_utf8_and_deep_nesting_are_typed_errors() {
        let mut decoder = NdjsonDecoder::default();
        let error = decoder
            .push("Welcome to the CLI! token=sk-abcdefghijklmnopqrstuvwxyz0123456789\n".as_bytes())
            .expect_err("banner");
        match &error {
            FramingError::NotJson { preview } => {
                assert!(preview.starts_with("Welcome"));
                assert!(!preview.contains("abcdefghijklmnop"), "{preview}");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error.reason_code(), "acp-stdout-not-protocol");

        let mut decoder = NdjsonDecoder::default();
        assert_eq!(
            decoder.push(&[0xff, 0xfe, b'\n']).expect_err("utf8"),
            FramingError::InvalidUtf8
        );

        let mut decoder = NdjsonDecoder::new(MAX_FRAME_BYTES, 4);
        let deep = b"{\"a\":{\"b\":{\"c\":{\"d\":{\"e\":1}}}}}\n";
        assert_eq!(
            decoder.push(deep).expect_err("deep"),
            FramingError::TooDeep { limit: 4 }
        );
        // Five nested objects plus the leaf value is depth six.
        let mut decoder = NdjsonDecoder::new(MAX_FRAME_BYTES, 6);
        assert_eq!(decoder.push(deep).expect("within budget").len(), 1);
        assert_eq!(depth_of(&json!([[[1]]])), 4);
        assert_eq!(depth_of(&json!("x")), 1);
    }

    #[test]
    fn encoder_produces_one_terminated_line() {
        let frame = encode_frame(&json!({"jsonrpc":"2.0","method":"m","params":{"text":"a\nb"}}))
            .expect("frame");
        assert_eq!(frame.iter().filter(|byte| **byte == b'\n').count(), 1);
        assert!(frame.ends_with(b"\n"));
        let huge = json!({"text": "z".repeat(MAX_FRAME_BYTES)});
        assert!(matches!(
            encode_frame(&huge),
            Err(FramingError::FrameTooLarge { .. })
        ));
    }
}
