## Context

Measured and verified before writing any of this:

- A four-cell notebook with one plot output: 122,675 characters on disk, 89 lines, longest line 120,039 characters. Through the `file` tool's `MAX_READ_LINE_CHARS` cap the model receives ~4,548 characters, of which 135 are cell source. 3% signal.
- Cell source is `["line one\n", "line two\n"]` — one JSON string per line, escaped. `execute_edit` matches exact strings against raw file bytes, so a two-line change spans two array elements and has no single unique match to anchor on.
- `serde_json` in this build sorts object keys on serialize: `{"nbformat":4,"cells":[],"zeta":1,"metadata":{},"alpha":2}` round-trips to `{"alpha":2,"cells":[],"metadata":{},"nbformat":4,"zeta":1}`. A parse-then-write of a whole notebook would therefore reorder every object in the file.
- `serde_json::value::RawValue` is available without a Cargo change, and preserves inner text verbatim: `{"z":2,"y":3}` came back in that order, unsorted.
- The `file` tool is narrowed to its read operation in `plan_mode_tool_catalog()` by declaring a separate definition whose `operation` enum is `["read"]`.

## Goals / Non-Goals

Goals:

- The model reads a notebook and sees the code, not the container.
- The model edits one cell without composing notebook JSON.
- A one-cell edit produces a one-cell diff.

Non-Goals:

- Executing cells, or talking to a kernel. This tool reads and writes a file.
- Rendering output images to the model. `add-agent-image-input` governs which tools return images and this is not one of them; adding it there is a separate change with its own budget question.
- Replacing `file` for notebooks. A caller that wants the raw JSON still has it.
- Notebook formats other than nbformat 4.

## Decisions

### D1: One tool with operations, not three tools

`notebook` takes an `operation` — `read`, `replace`, `insert`, `delete` — the way `file` takes read/write. Three or four separate catalog entries would spend four schemas on one file type, and every one of them carries the same path argument and the same addressing rules. The prompt cost of a tool is its schema on every request; one schema is the honest price for a capability this narrow.

### D2: Outputs are summarized, never carried

A read returns each output's type and a bounded amount of its text. An `image/png` output is reported as its media type and byte count and nothing else.

This is the entire reason reading is broken today, so it is worth being exact about: the 120,039-character line in the measurement above is one base64 PNG. Truncating it, which is what happens today, spends 2,000 characters of context to tell the model nothing. Naming it spends about 40 to tell it a plot exists and how big it is.

An error output keeps its `ename` and `evalue`, because a traceback is usually why the model is reading the notebook at all.

### D3: Address a cell by id, or by index

nbformat 4.5 gives every cell an `id`; 4.0–4.4 do not. So neither addressing mode works alone. The tool accepts exactly one of `cell_id` or `cell_index`, and the read result reports both for every cell.

An id survives edits and an index does not. Rather than pick for the caller, the read output says which is which, and the tool description says an id is preferred when the notebook has them.

### D4: Rewrite by splicing raw cell text, not by re-serializing

Since serde_json sorts keys, a parse-then-write reorders every object in the file — every cell, every output, every metadata block — turning a one-line change into a diff the size of the notebook. The alternative sometimes reached for is enabling `preserve_order`, but that changes `serde_json::Map` for the entire crate, affecting every wire format, artifact, and MCP payload in the build, to fix one tool.

Instead the cells array is parsed as `Vec<Box<RawValue>>` and the notebook's other top-level members are held the same way. Untouched cells and the notebook metadata are written back as the exact bytes they came in as; only the edited or inserted cell is rendered fresh. Top-level key order is the one thing not preserved — there are four keys, so the cost is bounded at a few lines, and they are written in nbformat's documented order.

### D5: Changing a code cell's source clears its outputs

Outputs describe one execution of one source. Once the source changes they describe something that no longer exists, and a model reading the notebook afterwards would take them as current.

Jupyter keeps stale outputs until a re-run, and this deliberately does not. Nothing here can re-run a cell, so keeping them would leave the file permanently claiming a result its code cannot produce. `execution_count` is cleared with them, which is exactly how nbformat marks a cell that has not been run. A markdown cell has neither and is unaffected.

### D6: Refuse anything that is not a notebook

A file that is not valid JSON, has no `cells` array, or declares an `nbformat` other than 4 is refused with the reason. The failure mode this avoids is specific: partially understanding a file and then writing a guess back over it, which for a notebook means destroying work that has no other copy.

## Risks / Trade-offs

- Clearing outputs on edit (D5) loses a result the user may have wanted to keep. Stated in the tool description so the model can say so, and the alternative — silently keeping results that contradict the code — is worse.
- Cell ids are not unique in a malformed notebook. The tool addresses the first match and reports the count when it is not one, rather than editing an arbitrary one of them.
- Reading a very large notebook is still bounded by the shared tool-output limit; the read reports how many cells it returned out of the total, so a truncated read is visible rather than silent.

## Migration Plan

None. New tool, no stored data, and no change to any existing tool's behavior.

## Open Questions

None.
