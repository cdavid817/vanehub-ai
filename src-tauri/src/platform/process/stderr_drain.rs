use std::io::{self, Read};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StderrCapture {
    retained: Vec<u8>,
    observed_bytes: u64,
    truncated: bool,
}

impl StderrCapture {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn retained(&self) -> &[u8] {
        &self.retained
    }

    pub(crate) fn observed_bytes(&self) -> u64 {
        self.observed_bytes
    }

    pub(crate) fn truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Debug, Error)]
pub(crate) enum StderrDrainError {
    #[error("failed while draining managed child stderr: {0}")]
    Io(#[from] io::Error),
    #[error("managed child stderr drain worker stopped unexpectedly")]
    WorkerStopped,
    #[error("managed child stderr drain exceeded its deadline")]
    TimedOut,
}

pub(crate) struct BlockingStderrDrain {
    worker: JoinHandle<io::Result<StderrCapture>>,
}

impl BlockingStderrDrain {
    pub(crate) fn spawn(reader: impl Read + Send + 'static, limit: usize) -> Self {
        Self {
            worker: thread::spawn(move || read_bounded(reader, limit)),
        }
    }

    pub(crate) fn finish(self) -> Result<StderrCapture, StderrDrainError> {
        self.worker
            .join()
            .map_err(|_| StderrDrainError::WorkerStopped)?
            .map_err(Into::into)
    }
}

pub(crate) struct TokioStderrDrain {
    worker: tokio::task::JoinHandle<io::Result<StderrCapture>>,
}

impl TokioStderrDrain {
    pub(crate) fn spawn(reader: impl AsyncRead + Unpin + Send + 'static, limit: usize) -> Self {
        Self {
            worker: tokio::spawn(read_bounded_async(reader, limit)),
        }
    }

    pub(crate) async fn finish(
        mut self,
        timeout: Duration,
    ) -> Result<StderrCapture, StderrDrainError> {
        match tokio::time::timeout(timeout, &mut self.worker).await {
            Ok(result) => result
                .map_err(|_| StderrDrainError::WorkerStopped)?
                .map_err(Into::into),
            Err(_) => {
                self.worker.abort();
                let _ = self.worker.await;
                Err(StderrDrainError::TimedOut)
            }
        }
    }
}

fn retain_chunk(capture: &mut StderrCapture, chunk: &[u8], limit: usize) {
    capture.observed_bytes = capture
        .observed_bytes
        .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
    let remaining = limit.saturating_sub(capture.retained.len());
    capture
        .retained
        .extend_from_slice(&chunk[..chunk.len().min(remaining)]);
    capture.truncated |= chunk.len() > remaining;
}

fn empty_capture(limit: usize) -> StderrCapture {
    StderrCapture {
        retained: Vec::with_capacity(limit.min(8 * 1024)),
        observed_bytes: 0,
        truncated: false,
    }
}

fn read_bounded(mut reader: impl Read, limit: usize) -> io::Result<StderrCapture> {
    let mut capture = empty_capture(limit);
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(capture);
        }
        retain_chunk(&mut capture, &buffer[..count], limit);
    }
}

async fn read_bounded_async(
    mut reader: impl AsyncRead + Unpin,
    limit: usize,
) -> io::Result<StderrCapture> {
    let mut capture = empty_capture(limit);
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(capture);
        }
        retain_chunk(&mut capture, &buffer[..count], limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::process::ManagedTokioChild;
    use std::collections::BTreeMap;
    use std::io::{Cursor, Write};
    use std::time::Instant;
    use tokio::io::AsyncWriteExt;

    const CAPTURE_LIMIT: usize = 64 * 1024;
    const NOISY_BYTES: usize = 512 * 1024;

    #[test]
    fn blocking_drain_consumes_all_input_and_retains_only_the_limit() {
        let drain = BlockingStderrDrain::spawn(Cursor::new(vec![b'x'; NOISY_BYTES]), CAPTURE_LIMIT);

        let capture = drain.finish().expect("bounded stderr capture");

        assert_eq!(capture.retained().len(), CAPTURE_LIMIT);
        assert_eq!(capture.observed_bytes(), NOISY_BYTES as u64);
        assert!(capture.truncated());
    }

    #[tokio::test]
    async fn noisy_managed_child_cannot_block_on_stderr_after_capture_limit() {
        let executable = std::env::current_exe().expect("test executable");
        let args = vec![
            "--ignored".to_string(),
            "--exact".to_string(),
            "platform::process::stderr_drain::tests::noisy_stderr_fixture".to_string(),
            "--nocapture".to_string(),
        ];
        let mut child =
            ManagedTokioChild::spawn(&executable.to_string_lossy(), &args, &BTreeMap::new())
                .expect("managed child");
        drop(child.take_stdin().expect("stdin"));
        let mut stdout = child.take_stdout().expect("stdout");
        let stderr = child.take_stderr().expect("stderr");
        let stdout_drain =
            tokio::spawn(async move { tokio::io::copy(&mut stdout, &mut tokio::io::sink()).await });
        let stderr_drain = TokioStderrDrain::spawn(stderr, CAPTURE_LIMIT);

        let status = child
            .wait_until(Instant::now() + Duration::from_secs(5))
            .await
            .expect("bounded wait")
            .expect("noisy child exited");
        let capture = stderr_drain
            .finish(Duration::from_secs(1))
            .await
            .expect("stderr drain");
        stdout_drain
            .await
            .expect("stdout drain task")
            .expect("stdout");

        assert!(status.success(), "noisy fixture failed");
        assert_eq!(capture.retained().len(), CAPTURE_LIMIT);
        assert!(capture.observed_bytes() >= NOISY_BYTES as u64);
        assert!(capture.truncated());
    }

    #[tokio::test]
    async fn timed_out_drain_is_aborted_and_awaited() {
        let (mut writer, reader) = tokio::io::duplex(16);
        writer
            .write_all(b"still-open")
            .await
            .expect("write fixture");
        let drain = TokioStderrDrain::spawn(reader, CAPTURE_LIMIT);

        let error = drain
            .finish(Duration::from_millis(10))
            .await
            .expect_err("open writer keeps drain pending");

        assert!(matches!(error, StderrDrainError::TimedOut));
    }

    #[test]
    #[ignore = "spawned only by the noisy managed-child test"]
    fn noisy_stderr_fixture() {
        std::io::stderr()
            .write_all(&vec![b'x'; NOISY_BYTES])
            .expect("write noisy stderr");
    }
}
