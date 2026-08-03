use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

const READ_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Clone, Default)]
pub(super) struct BoundedLineStatus {
    exceeded: Arc<AtomicBool>,
}

impl BoundedLineStatus {
    pub(super) fn exceeded(&self) -> bool {
        self.exceeded.load(Ordering::Acquire)
    }
}

pub(super) struct BoundedLineReader<R> {
    inner: R,
    maximum: usize,
    current: usize,
    failed: bool,
    status: BoundedLineStatus,
    buffer: [u8; READ_CHUNK_BYTES],
}

impl<R> BoundedLineReader<R> {
    pub(super) fn new(inner: R, maximum: usize) -> (Self, BoundedLineStatus) {
        let status = BoundedLineStatus::default();
        (
            Self {
                inner,
                maximum,
                current: 0,
                failed: false,
                status: status.clone(),
                buffer: [0; READ_CHUNK_BYTES],
            },
            status,
        )
    }

    fn limit_error(&mut self) -> io::Error {
        self.failed = true;
        self.status.exceeded.store(true, Ordering::Release);
        io::Error::new(
            io::ErrorKind::InvalidData,
            "MCP stdio frame exceeded byte limit",
        )
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BoundedLineReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.failed {
            return Poll::Ready(Err(this.limit_error()));
        }
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let bytes_until_limit = this.maximum.saturating_sub(this.current).saturating_add(1);
        let capacity = output
            .remaining()
            .min(READ_CHUNK_BYTES)
            .min(bytes_until_limit.max(1));
        let mut scratch = ReadBuf::new(&mut this.buffer[..capacity]);
        match Pin::new(&mut this.inner).poll_read(context, &mut scratch) {
            Poll::Ready(Ok(())) => {
                let read = scratch.filled().len();
                let mut current = this.current;
                let mut exceeded = false;
                for byte in &this.buffer[..read] {
                    if *byte == b'\n' {
                        current = 0;
                    } else {
                        current = current.saturating_add(1);
                        if current > this.maximum {
                            exceeded = true;
                            break;
                        }
                    }
                }
                if exceeded {
                    return Poll::Ready(Err(this.limit_error()));
                }
                this.current = current;
                output.put_slice(&this.buffer[..read]);
                Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::application::McpLimits;
    use std::io::Cursor;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn exact_boundary_and_multiple_lines_are_accepted() {
        let maximum = McpLimits::DEFAULT.protocol_message_bytes;
        let input = [vec![b'a'; maximum], b"\nsmall\n".to_vec()].concat();
        let (mut reader, status) = BoundedLineReader::new(Cursor::new(input.clone()), maximum);
        let mut received = Vec::new();

        reader
            .read_to_end(&mut received)
            .await
            .expect("bounded lines");

        assert_eq!(received, input);
        assert!(!status.exceeded());
    }

    #[tokio::test]
    async fn limit_plus_one_is_rejected_while_reading() {
        let maximum = McpLimits::DEFAULT.protocol_message_bytes;
        let (mut reader, status) =
            BoundedLineReader::new(Cursor::new(vec![b'x'; maximum + 1]), maximum);
        let mut received = Vec::new();

        let error = reader
            .read_to_end(&mut received)
            .await
            .expect_err("oversized line");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(status.exceeded());
    }
}
