//! Shared, provider-agnostic accumulation of streamed tool-call fragments into complete
//! `ToolUseBlock`s. Both wire formats stream a tool call's JSON input/arguments across multiple
//! SSE chunks, keyed by a numeric index — this is the one piece of genuinely stateful logic in
//! an otherwise pure-function translation layer, so it is isolated here and unit-tested alone.

use crate::contexts::agent_runtime::application::ToolUseBlock;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
struct PendingToolCall {
    id: String,
    name: String,
    partial_json: String,
}

#[derive(Debug, Default)]
pub(crate) struct ToolCallAccumulator {
    pending: BTreeMap<u32, PendingToolCall>,
    completed: Vec<ToolUseBlock>,
}

impl ToolCallAccumulator {
    /// Begins accumulating a new tool call at `index`. If `id`/`name` are not yet known for this
    /// index (OpenAI only sends them on the first fragment), pass empty strings — `id`/`name` are
    /// filled in from later calls to `start` for the same index only if they are non-empty, so a
    /// blank second call never clobbers values already captured.
    pub(crate) fn start(&mut self, index: u32, id: &str, name: &str) {
        let entry = self.pending.entry(index).or_default();
        if !id.is_empty() {
            entry.id = id.to_string();
        }
        if !name.is_empty() {
            entry.name = name.to_string();
        }
    }

    /// Appends a fragment of the tool call's JSON input/arguments string at `index`.
    pub(crate) fn append_json(&mut self, index: u32, fragment: &str) {
        self.pending
            .entry(index)
            .or_default()
            .partial_json
            .push_str(fragment);
    }

    /// Finalizes the tool call at `index`: parses its accumulated JSON and moves it into the
    /// completed list. A no-op if `index` has no pending entry (defensive against malformed or
    /// out-of-order streams).
    pub(crate) fn finish(&mut self, index: u32) {
        if let Some(pending) = self.pending.remove(&index) {
            self.completed.push(pending.into_block());
        }
    }

    /// Finalizes every still-pending tool call. Used by wire formats (OpenAI) whose stream has
    /// no per-call completion signal — only a single end-of-response marker for all of them.
    pub(crate) fn finish_all_pending(&mut self) {
        let indices: Vec<u32> = self.pending.keys().copied().collect();
        for index in indices {
            self.finish(index);
        }
    }

    /// Drains and returns every tool call finalized so far, in the order they completed.
    pub(crate) fn take_completed(&mut self) -> Vec<ToolUseBlock> {
        std::mem::take(&mut self.completed)
    }
}

impl PendingToolCall {
    fn into_block(self) -> ToolUseBlock {
        let input = serde_json::from_str::<Value>(&self.partial_json).ok();
        ToolUseBlock {
            id: self.id,
            name: self.name,
            input,
            output: None,
            status: "pending".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_fragments_and_finishes_by_index() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.start(0, "toolu_1", "shell");
        accumulator.append_json(0, "{\"comm");
        accumulator.append_json(0, "and\":\"ls\"}");
        accumulator.finish(0);

        let completed = accumulator.take_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, "toolu_1");
        assert_eq!(completed[0].name, "shell");
        assert_eq!(
            completed[0].input,
            Some(serde_json::json!({"command": "ls"}))
        );
    }

    #[test]
    fn tracks_multiple_indices_independently() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.start(0, "call_1", "shell");
        accumulator.start(1, "call_2", "file");
        accumulator.append_json(0, "{\"command\":\"ls\"}");
        accumulator.append_json(1, "{\"operation\":\"read\",\"path\":\"a.txt\"}");
        accumulator.finish(0);
        accumulator.finish(1);

        let completed = accumulator.take_completed();
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].name, "shell");
        assert_eq!(completed[1].name, "file");
    }

    #[test]
    fn later_id_and_name_fragments_do_not_clobber_earlier_ones_with_blanks() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.start(0, "call_1", "shell");
        // OpenAI-style: subsequent fragments carry only `index`, no id/name.
        accumulator.start(0, "", "");
        accumulator.append_json(0, "{}");
        accumulator.finish(0);

        let completed = accumulator.take_completed();
        assert_eq!(completed[0].id, "call_1");
        assert_eq!(completed[0].name, "shell");
    }

    #[test]
    fn finish_all_pending_flushes_every_incomplete_index() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.start(0, "call_1", "shell");
        accumulator.append_json(0, "{\"command\":\"ls\"}");
        accumulator.start(1, "call_2", "file");
        accumulator.append_json(1, "{\"operation\":\"read\",\"path\":\"a.txt\"}");
        // No explicit finish() calls — simulating OpenAI's lack of a per-call completion signal.
        accumulator.finish_all_pending();

        assert_eq!(accumulator.take_completed().len(), 2);
    }

    #[test]
    fn finish_on_unknown_index_is_a_harmless_no_op() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.finish(0);
        assert!(accumulator.take_completed().is_empty());
    }

    #[test]
    fn malformed_json_produces_a_block_with_no_parsed_input() {
        let mut accumulator = ToolCallAccumulator::default();
        accumulator.start(0, "call_1", "shell");
        accumulator.append_json(0, "not json");
        accumulator.finish(0);

        let completed = accumulator.take_completed();
        assert_eq!(completed[0].input, None);
    }
}
