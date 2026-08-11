use super::lsp_framing::{FrameLimits, LspFrameError, LspFrameReader, LspFrameWriter};
use std::collections::VecDeque;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

fn limits(max_header_bytes: usize, max_payload_bytes: usize) -> FrameLimits {
    FrameLimits::new(max_header_bytes, max_payload_bytes).expect("valid limits")
}

fn frame(header_name: &str, payload: &[u8]) -> Vec<u8> {
    let mut bytes = format!("{header_name}: {}\r\n\r\n", payload.len()).into_bytes();
    bytes.extend_from_slice(payload);
    bytes
}

#[tokio::test]
async fn reader_reassembles_headers_and_payloads_split_across_partial_reads() {
    let input = ChunkedReader::new([
        b"Con".as_slice(),
        b"tent-Len".as_slice(),
        b"gth: 7\r".as_slice(),
        b"\n\r\n{\"".as_slice(),
        b"a\":1}".as_slice(),
    ]);
    let mut reader = LspFrameReader::new(input, limits(128, 64));

    assert_eq!(
        reader.read_frame().await.expect("frame"),
        Some(br#"{"a":1}"#.to_vec())
    );
    assert_eq!(reader.read_frame().await.expect("clean EOF"), None);
}

#[tokio::test]
async fn reader_preserves_multiple_back_to_back_frames() {
    let mut input = frame("Content-Length", br#"{"id":1}"#);
    input.extend(frame("Content-Length", br#"{"id":2}"#));
    let mut reader = LspFrameReader::new(Cursor::new(input), limits(128, 64));

    assert_eq!(
        reader.read_frame().await.expect("first"),
        Some(br#"{"id":1}"#.to_vec())
    );
    assert_eq!(
        reader.read_frame().await.expect("second"),
        Some(br#"{"id":2}"#.to_vec())
    );
    assert_eq!(reader.read_frame().await.expect("EOF"), None);
}

#[tokio::test]
async fn content_length_header_name_is_ascii_case_insensitive() {
    let input = frame("cOnTeNt-LeNgTh", b"{}");
    let mut reader = LspFrameReader::new(Cursor::new(input), limits(128, 8));

    assert_eq!(
        reader.read_frame().await.expect("frame"),
        Some(b"{}".to_vec())
    );
}

#[tokio::test]
async fn malformed_or_ambiguous_headers_are_rejected() {
    for input in [
        b"Content-Type: application/json\r\n\r\n{}".as_slice(),
        b"Content-Length: nope\r\n\r\n".as_slice(),
        b"Content-Length 2\r\n\r\n{}".as_slice(),
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".as_slice(),
    ] {
        let mut reader = LspFrameReader::new(Cursor::new(input), limits(128, 8));
        assert_eq!(reader.read_frame().await, Err(LspFrameError::InvalidHeader));
    }
}

#[tokio::test]
async fn header_bytes_are_hard_bounded_before_the_terminator() {
    let input = b"X-Fill: 123456789\r\nContent-Length: 2\r\n\r\n{}";
    let mut reader = LspFrameReader::new(Cursor::new(input), limits(16, 8));

    assert_eq!(
        reader.read_frame().await,
        Err(LspFrameError::HeaderTooLarge)
    );
}

#[tokio::test]
async fn unexpected_eof_is_distinct_from_clean_stream_completion() {
    let mut header_eof = LspFrameReader::new(Cursor::new(b"Content-Length: 2\r\n"), limits(128, 8));
    let mut body_eof =
        LspFrameReader::new(Cursor::new(b"Content-Length: 4\r\n\r\n{}"), limits(128, 8));

    assert_eq!(
        header_eof.read_frame().await,
        Err(LspFrameError::UnexpectedEof)
    );
    assert_eq!(
        body_eof.read_frame().await,
        Err(LspFrameError::UnexpectedEof)
    );
}

#[tokio::test]
async fn payload_at_limit_is_accepted_and_payload_over_limit_is_rejected() {
    let mut exact = LspFrameReader::new(
        Cursor::new(frame("Content-Length", b"1234")),
        limits(128, 4),
    );
    let mut oversized =
        LspFrameReader::new(Cursor::new(b"Content-Length: 5\r\n\r\n"), limits(128, 4));

    assert_eq!(
        exact.read_frame().await.expect("exact limit"),
        Some(b"1234".to_vec())
    );
    assert_eq!(
        oversized.read_frame().await,
        Err(LspFrameError::PayloadTooLarge)
    );
}

#[tokio::test]
async fn hostile_declared_frame_size_is_rejected_before_payload_allocation() {
    let input = b"Content-Length: 1000000000\r\n\r\n";
    let mut reader = LspFrameReader::new(Cursor::new(input), limits(128, 64));

    assert_eq!(
        reader.read_frame().await,
        Err(LspFrameError::PayloadTooLarge)
    );
}

#[tokio::test]
async fn writer_serializes_a_complete_frame_and_enforces_the_payload_limit() {
    let (server, mut capture) = tokio::io::duplex(256);
    let mut writer = LspFrameWriter::new(server, 4).expect("writer");
    writer.write_frame(b"1234").await.expect("exact frame");
    assert_eq!(
        writer.write_frame(b"12345").await,
        Err(LspFrameError::PayloadTooLarge)
    );
    drop(writer);

    let mut bytes = Vec::new();
    capture
        .read_to_end(&mut bytes)
        .await
        .expect("capture frame");
    assert_eq!(bytes, b"Content-Length: 4\r\n\r\n1234");
}

struct ChunkedReader {
    chunks: VecDeque<Vec<u8>>,
}

impl ChunkedReader {
    fn new<'a>(chunks: impl IntoIterator<Item = &'a [u8]>) -> Self {
        Self {
            chunks: chunks.into_iter().map(<[u8]>::to_vec).collect(),
        }
    }
}

impl AsyncRead for ChunkedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let Some(mut chunk) = self.chunks.pop_front() else {
            return Poll::Ready(Ok(()));
        };
        let count = buffer.remaining().min(chunk.len());
        buffer.put_slice(&chunk[..count]);
        if count < chunk.len() {
            chunk.drain(..count);
            self.chunks.push_front(chunk);
        }
        Poll::Ready(Ok(()))
    }
}
