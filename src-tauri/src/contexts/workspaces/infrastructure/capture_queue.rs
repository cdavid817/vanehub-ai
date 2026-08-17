use crate::contexts::workspaces::domain::{TerminalOutputChunk, TerminalOutputSource};
use std::collections::VecDeque;
use std::sync::Mutex;

pub(crate) struct BoundedCaptureQueue {
    capacity: usize,
    chunks: Mutex<VecDeque<TerminalOutputChunk>>,
    dropped: Mutex<bool>,
}

impl BoundedCaptureQueue {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            chunks: Mutex::new(VecDeque::new()),
            dropped: Mutex::new(false),
        }
    }
    pub(crate) fn push(&self, chunk: TerminalOutputChunk) {
        let Ok(mut chunks) = self.chunks.lock() else {
            return;
        };
        if chunks.len() >= self.capacity {
            chunks.pop_front();
            if let Ok(mut dropped) = self.dropped.lock() {
                *dropped = true;
            }
        }
        chunks.push_back(chunk);
    }
    pub(crate) fn drain_batch(&self, limit: usize) -> Vec<TerminalOutputChunk> {
        let mut result = Vec::new();
        if let Ok(mut dropped) = self.dropped.lock() {
            if *dropped {
                result.push(TerminalOutputChunk {
                    stream_id: "capture".to_string(),
                    sequence: 0,
                    source: TerminalOutputSource::Gap,
                    content: "[capture gap]".to_string(),
                });
                *dropped = false;
            }
        }
        if let Ok(mut chunks) = self.chunks.lock() {
            let count = limit.max(1).min(chunks.len());
            result.extend(chunks.drain(..count));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_emits_one_gap_marker() {
        let queue = BoundedCaptureQueue::new(1);
        queue.push(TerminalOutputChunk {
            stream_id: "s".into(),
            sequence: 1,
            source: TerminalOutputSource::Pty,
            content: "one".into(),
        });
        queue.push(TerminalOutputChunk {
            stream_id: "s".into(),
            sequence: 2,
            source: TerminalOutputSource::Pty,
            content: "two".into(),
        });
        let batch = queue.drain_batch(10);
        assert_eq!(batch[0].source, TerminalOutputSource::Gap);
        assert_eq!(
            batch
                .iter()
                .filter(|chunk| chunk.source == TerminalOutputSource::Gap)
                .count(),
            1
        );
    }

    #[test]
    fn performance_long_terminal_queue_remains_bounded_and_emits_one_gap() {
        const CAPACITY: usize = 256;
        const CHUNKS: usize = 4_096;
        const CHUNK_BYTES: usize = 4_096;
        let queue = BoundedCaptureQueue::new(CAPACITY);
        for sequence in 0..CHUNKS {
            queue.push(TerminalOutputChunk {
                stream_id: "performance-stream".into(),
                sequence: sequence as u64,
                source: TerminalOutputSource::Pty,
                content: "x".repeat(CHUNK_BYTES),
            });
        }
        let first = queue.drain_batch(32);
        assert_eq!(first.len(), 33);
        assert_eq!(first[0].source, TerminalOutputSource::Gap);
        assert_eq!(
            first
                .iter()
                .filter(|chunk| chunk.source == TerminalOutputSource::Gap)
                .count(),
            1
        );
        assert!(first
            .iter()
            .skip(1)
            .all(|chunk| chunk.content.len() == CHUNK_BYTES));
        let mut retained = first.len() - 1;
        loop {
            let batch = queue.drain_batch(32);
            if batch.is_empty() {
                break;
            }
            assert!(batch
                .iter()
                .all(|chunk| chunk.source != TerminalOutputSource::Gap));
            retained += batch.len();
        }
        assert_eq!(retained, CAPACITY);
        eprintln!(
            "TERMINAL_QUEUE_PERFORMANCE dataset=terminal-long@1 inputChunks={CHUNKS} retainedChunks={retained} chunkBytes={CHUNK_BYTES} gapMarkers=1"
        );
    }
}
