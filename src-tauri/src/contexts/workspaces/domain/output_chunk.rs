use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalOutputSource { Pty, QuickCommand, Gap }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalOutputChunk {
    pub(crate) stream_id: String,
    pub(crate) sequence: u64,
    pub(crate) source: TerminalOutputSource,
    pub(crate) content: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum TerminalOutputError { #[error("terminal output stream id is invalid")] InvalidStream, #[error("terminal output chunk exceeds the bounded size")] TooLarge }

impl TerminalOutputChunk {
    pub(crate) fn normalize(stream_id: impl Into<String>, sequence: u64, source: TerminalOutputSource, bytes: &[u8]) -> Result<Self, TerminalOutputError> {
        let stream_id = stream_id.into();
        if stream_id.trim().is_empty() || stream_id.len() > 128 { return Err(TerminalOutputError::InvalidStream); }
        if bytes.len() > 32 * 1024 { return Err(TerminalOutputError::TooLarge); }
        let content = String::from_utf8_lossy(bytes).replace('\u{1b}', "");
        Ok(Self { stream_id, sequence, source, content })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_invalid_utf8_and_rejects_oversized_chunks() {
        let chunk = TerminalOutputChunk::normalize("stream", 1, TerminalOutputSource::Pty, &[0xff, b'o', b'k']).expect("chunk");
        assert!(chunk.content.contains('\u{fffd}'));
        assert!(TerminalOutputChunk::normalize("stream", 2, TerminalOutputSource::Pty, &vec![b'x'; 32 * 1024 + 1]).is_err());
    }
}
