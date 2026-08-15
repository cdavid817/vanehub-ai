## Why

Referenced files are injected into the Agent prompt as full text. A referenced file may be up to 1 MB, so referencing one large file can consume most of a context window, and referencing several can exhaust it outright. The user often means a specific function or block, not the whole file, and has no way to say so.

Injecting the whole file is nevertheless the right strategy and stays: API-mode agents have no file-reading tool at all, so replacing content with a path would silently break file references on that half of the runtimes. The way to spend less context without losing the capability is to inject **less of the file**, not to stop injecting it.

This change lets a reference carry a line range. The composer accepts `@path:10-50`, the range travels with the reference through persistence and history, and prompt assembly inlines only those lines.

## What Changes

- **References carry an optional line range** — `startLine`/`endLine` are added to the chat file reference type in TypeScript, to the `FileReference` domain model, and to the command DTO. Absent means the whole file, which is what every existing reference means.
- **`@path:10-50` in the composer** — mention completion accepts a trailing `:start-end` or `:line` suffix. Candidate search matches on the path portion only, so completion keeps working while the range is being typed. Selecting a candidate still inserts a plain path; the suffix is typed by the user.
- **Prompt assembly inlines only the requested lines** — the injected block is labelled with its range and carries 1-based line numbers so the Agent can cite positions that match the user's editor. A reference without a range is inlined exactly as it is today.
- **Ranges are validated in the domain** — both bounds present or both absent, `startLine >= 1`, `endLine >= startLine`. A range that runs past the end of the file is clamped rather than rejected, because a file can be edited after it is referenced and a stale upper bound should not fail the send.
- **Reference identity becomes (path, range)** — `FileReferenceSet` deduplicates on the path *and* the range instead of the path alone, so two regions of one file can be referenced together. An exact duplicate is still rejected, and `MAX_FILE_REFERENCES` remains the overall ceiling. **BREAKING** at the domain level only: no persisted data changes meaning, since every stored reference has no range and therefore keeps its old identity.
- **Chips show the range** — both the composer chip and the message-history chip label a ranged reference; removal targets the reference identity rather than the path, which a path alone can no longer address.

Not in scope: the file preview modal with click-to-select line anchors, drag-and-drop, and clipboard paste. Those are proposed separately and will produce ranges through the same data path this change establishes.

## Capabilities

### New Capabilities

None. The behavior belongs to the existing chat file reference capability.

### Modified Capabilities

- `chat-experience`: The "Chat file references" requirement gains line-range behavior — how a range is expressed in the composer, how it is validated, how it bounds prompt injection, how it participates in reference identity, and how it is displayed on chips. The existing scenarios for candidate search, unsafe-reference rejection, and metadata persistence keep their meaning.

## Impact

**Runtimes:** Both. Range parsing, chip rendering, and the service payload are shared; prompt assembly and validation are native. The Web/mock adapter carries the wider payload so browser mode keeps composing valid messages.

**Adapter boundary:** Unchanged. No new command and no new service method — `sendMessage` already carries `file_references`, and the range rides inside that existing structure. React components gain no direct `invoke()` usage.

**Native layer:**
- `FileReference` gains two optional fields plus range validation; `FileReferenceSet` changes its dedup key.
- `compose_prompt` slices the file it already reads. Path containment, oversize, and binary safeguards are untouched — a range narrows what is injected and never widens what can be read.
- The `send_message` DTO and its mapper carry the two fields through.

**Persistence:** No schema migration. `messages.file_references` is a JSON `TEXT` column, so the added fields deserialize as absent on every existing row and existing references keep resolving as whole-file.

**Frontend:**
- `ChatFileReference` gains the two fields; mention parsing splits the path from the range suffix.
- Chip removal moves from path-keyed to identity-keyed in the composer, the layout model, and `MessageItem`.
- `ChatInputBox.tsx` is at 180 of the 300-line limit after the previous change, and mention state already lives in a hook, so range parsing lands in that hook rather than the component.

**Contracts and CI:** No new command to register, but `ChatFileReference` is a contract type, so `npm run contracts:check` covers the added fields. Existing tests that build file references or assert chip removal need the wider shape.
