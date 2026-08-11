use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

pub(crate) const DEFAULT_MAX_HEADER_BYTES: usize = 8 * 1024;
pub(crate) const DEFAULT_MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameLimits {
    max_header_bytes: usize,
    max_payload_bytes: usize,
}

impl FrameLimits {
    pub(crate) fn new(
        max_header_bytes: usize,
        max_payload_bytes: usize,
    ) -> Result<Self, LspFrameError> {
        if max_header_bytes < 4 || max_payload_bytes == 0 {
            return Err(LspFrameError::InvalidLimits);
        }
        Ok(Self {
            max_header_bytes,
            max_payload_bytes,
        })
    }

    pub(crate) const fn max_payload_bytes(self) -> usize {
        self.max_payload_bytes
    }
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_header_bytes: DEFAULT_MAX_HEADER_BYTES,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LspFrameError {
    #[error("LSP frame limits are invalid")]
    InvalidLimits,
    #[error("LSP header exceeds its byte limit")]
    HeaderTooLarge,
    #[error("LSP header is malformed")]
    InvalidHeader,
    #[error("LSP payload exceeds its byte limit")]
    PayloadTooLarge,
    #[error("LSP stream ended before the declared frame completed")]
    UnexpectedEof,
    #[error("LSP stream I/O failed")]
    Io,
}

pub(crate) struct LspFrameReader<R> {
    reader: BufReader<R>,
    limits: FrameLimits,
}

impl<R: AsyncRead + Unpin> LspFrameReader<R> {
    pub(crate) fn new(reader: R, limits: FrameLimits) -> Self {
        Self {
            reader: BufReader::new(reader),
            limits,
        }
    }

    pub(crate) async fn read_frame(&mut self) -> Result<Option<Vec<u8>>, LspFrameError> {
        let Some(header) = self.read_header().await? else {
            return Ok(None);
        };
        let content_length = parse_content_length(&header)?;
        if content_length > self.limits.max_payload_bytes {
            return Err(LspFrameError::PayloadTooLarge);
        }
        let mut payload = vec![0_u8; content_length];
        self.reader
            .read_exact(&mut payload)
            .await
            .map_err(map_read_error)?;
        Ok(Some(payload))
    }

    async fn read_header(&mut self) -> Result<Option<Vec<u8>>, LspFrameError> {
        let mut header = Vec::with_capacity(self.limits.max_header_bytes.min(256));
        loop {
            let mut byte = [0_u8; 1];
            let read = self
                .reader
                .read(&mut byte)
                .await
                .map_err(|_| LspFrameError::Io)?;
            if read == 0 {
                return if header.is_empty() {
                    Ok(None)
                } else {
                    Err(LspFrameError::UnexpectedEof)
                };
            }
            header.push(byte[0]);
            if header.len() > self.limits.max_header_bytes {
                return Err(LspFrameError::HeaderTooLarge);
            }
            if header.ends_with(b"\r\n\r\n") {
                header.truncate(header.len() - 4);
                return Ok(Some(header));
            }
        }
    }
}

pub(crate) struct LspFrameWriter<W> {
    writer: W,
    max_payload_bytes: usize,
}

impl<W: AsyncWrite + Unpin> LspFrameWriter<W> {
    pub(crate) fn new(writer: W, max_payload_bytes: usize) -> Result<Self, LspFrameError> {
        if max_payload_bytes == 0 {
            return Err(LspFrameError::InvalidLimits);
        }
        Ok(Self {
            writer,
            max_payload_bytes,
        })
    }

    pub(crate) async fn write_frame(&mut self, payload: &[u8]) -> Result<(), LspFrameError> {
        if payload.len() > self.max_payload_bytes {
            return Err(LspFrameError::PayloadTooLarge);
        }
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());
        self.writer
            .write_all(header.as_bytes())
            .await
            .map_err(|_| LspFrameError::Io)?;
        self.writer
            .write_all(payload)
            .await
            .map_err(|_| LspFrameError::Io)?;
        self.writer.flush().await.map_err(|_| LspFrameError::Io)
    }
}

fn parse_content_length(header: &[u8]) -> Result<usize, LspFrameError> {
    let header = std::str::from_utf8(header).map_err(|_| LspFrameError::InvalidHeader)?;
    let mut content_length = None;
    for line in header.split("\r\n") {
        let (name, value) = line.split_once(':').ok_or(LspFrameError::InvalidHeader)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LspFrameError::InvalidHeader);
        }
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                return Err(LspFrameError::InvalidHeader);
            }
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(LspFrameError::InvalidHeader);
            }
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| LspFrameError::PayloadTooLarge)?,
            );
        }
    }
    content_length.ok_or(LspFrameError::InvalidHeader)
}

fn map_read_error(error: std::io::Error) -> LspFrameError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        LspFrameError::UnexpectedEof
    } else {
        LspFrameError::Io
    }
}
