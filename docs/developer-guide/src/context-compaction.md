# Context compaction

A generation's accumulated turns are measured by **summed character count**, not by provider-reported token counts. When the running total exceeds a fixed threshold, the runtime compacts before sending the next request. This deliberately avoids depending on real provider-reported token counts to decide when to compact.

## When compaction triggers

- Below the threshold → the request is sent unmodified.
- The session's conversation history alone exceeds the threshold → compact before the first request of that generation.
- Turns accumulated during a tool-use loop (tool-call results) push the total over the threshold → compact before the loop's next request.

## Summarization compaction

When compaction triggers, the runtime keeps a fixed number of the most recent turns **verbatim** and replaces all older turns with a single synthetic turn carrying a model-generated summary of them. The summarization call is a single provider call over the turns older than the kept window; the summarization call does not declare tools.

## Where the design lives

This chapter orients contributors. The authoritative requirements — the character-count trigger and the summarization compaction — live in the spec.

- [openspec/specs/agent-context-compaction](../../../openspec/specs/agent-context-compaction/spec.md)

Compaction runs in the `agent_runtime` bounded context, in the same path as the tool-use loop described in [Tool registry and execution](tool-registry.md).
