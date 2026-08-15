## 1. Reading a notebook as cells

- [x] 1.1 Parse a notebook into cells, keeping each cell's raw text alongside its decoded fields.
- [x] 1.2 Render each cell as index, id, type, and source, joining the per-line source array back into text.
- [x] 1.3 Summarize each output by type, keeping bounded text and an error's `ename`/`evalue`.
- [x] 1.4 Report an image output by media type and byte count, never by its bytes.
- [x] 1.5 Bound the whole read by the shared tool-output limit and say how many cells were returned of the total.

## 2. Editing a cell

- [x] 2.1 Replace a cell's source, addressed by id or index.
- [x] 2.2 Insert a new cell of a given type before or after an addressed cell, and at the start or end.
- [x] 2.3 Delete an addressed cell.
- [x] 2.4 Clear a code cell's outputs and execution count when its source changes.
- [x] 2.5 Require exactly one of `cell_id` or `cell_index`, and report an ambiguous or missing address rather than guessing.

## 3. Writing it back

- [x] 3.1 Write untouched cells and the notebook's other members as their original bytes.
- [x] 3.2 Render only the edited or inserted cell, giving a new cell an id when the notebook uses them.
- [x] 3.3 Write through the same atomic replace the edit tool uses, so an interrupted write cannot truncate a notebook.
- [x] 3.4 Refuse a file that is not valid JSON, has no cells array, or is not nbformat 4, without writing anything.

## 4. Tool surface

- [x] 4.1 Declare the tool with its four operations, appended after the existing catalog entries.
- [x] 4.2 Offer read-only notebook access in plan mode, matching how the file tool is narrowed there.
- [x] 4.3 Register a handler and classify its permission action per operation: reading is a read, the rest are writes.
- [x] 4.4 Apply the same workspace boundary, hidden-path, and size rules the file tools apply.

## 5. Tests

- [x] 5.1 A read returns cell source and never an output image's bytes.
- [x] 5.2 An error output keeps the information a model reads a notebook to find.
- [x] 5.3 Replace, insert, and delete each produce a correct notebook that still parses.
- [x] 5.4 An untouched cell is byte-identical after an edit, including its key order.
- [x] 5.5 Editing a code cell's source clears its outputs and execution count; a markdown cell is unaffected.
- [x] 5.6 Addressing by id and by index select the same cell, and supplying both or neither is refused.
- [x] 5.7 A non-notebook, a non-nbformat-4 file, and malformed JSON are each refused without writing.
- [x] 5.8 Plan mode offers reading only, and a write operation is refused there.
- [x] 5.9 Path escape, hidden paths, and oversized files are refused as they are for the file tools.

## 6. Validation

- [x] 6.1 `npm run lint:ci`
- [x] 6.2 `npm run test`
- [x] 6.3 `npm run build`
- [x] 6.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 6.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.8 `openspec validate add-agent-notebook-tool --strict`
- [x] 6.9 `openspec validate --specs --strict`

## Status

`notebook` reads a notebook as cells and edits it one cell at a time. Reading returns index, id,
type, and source with outputs summarized; an image output is named and measured, never carried.
Replace, insert, and delete address a cell by id or index. It is appended last in the catalog so the
prompt-cache prefix of every existing generation is unchanged, and plan mode declares a read-only
variant the way it already does for the file tool.

Three measurements drove the design rather than assumptions. A four-cell notebook with one plot is
122,675 characters on disk, of which the model receives ~4,548 through the file tool's per-line cap
and only 135 are cell source -- 3% signal. `serde_json` here sorts object keys, verified by probe,
so a parse-then-write would reorder every object in the file. And `RawValue` is already available
without a Cargo change and preserves inner text verbatim, which is what makes the splice-based
rewrite possible: enabling `preserve_order` would have changed `serde_json::Map` for every wire
format, artifact, and MCP payload in the build to fix one tool.

One bug worth recording. `NOTEBOOK_TOOL_NAME` was used in a match arm before it was imported, and
an unimported identifier in pattern position is an irrefutable binding, not a constant -- so the arm
silently matched every tool name and made every later arm dead. It compiled. The only signal was a
`unreachable pattern` warning, which is why the clippy gate is run rather than just `cargo check`.

The seven hardcoded catalog-count tests broke as expected when the tool was added. One near-miss
during that fix: `providers/tests.rs` has an `assert_eq!(tools.len(), 10)` that counts tool
*lifecycle events parsed from a fixture stream*, not catalog entries, and bumping it was wrong.
