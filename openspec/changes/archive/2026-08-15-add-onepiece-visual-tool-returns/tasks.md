## 1. Result envelope image channel

- [x] 1.1 Carry the image as an Artifact id in the envelope's existing metadata rather than adding a field, keeping the change additive across ninety-odd construction sites.
- [x] 1.2 Surface it through `execute_registered_native_tool` into the executed-call tuple `build_reply_turns` already accepts.
- [x] 1.3 Keep base64 out of the tool output the transcript persists, and pin that with a test.
- [x] 1.4 Land this step with the existing extended-tool tests green, before either adapter changes.

## 2. Tool surfaces

- [x] 2.1 Return the captured image from the Browser screenshot operation alongside its existing Artifact reference.
- [x] 2.2 Return the page OCR read alongside its existing text and Artifact reference.
- [x] 2.3 Route both through the existing image preparation so they inherit reviewed types, bounds, downscaling, and the per-request budget.
- [x] 2.4 Degrade both to their current non-image results on a text-only model.
- [x] 2.5 Carry the Artifact reference into the transcript for every image-returning tool.

## 3. Tests

- [x] 3.1 Envelope tests: an image reaches the reply turns, and no base64 reaches the tool output or the persisted transcript.
- [x] 3.2 Screenshot and OCR image-return tests, including the text-only degradation path.
- [x] 3.3 A bound test proving a produced image goes through the same downscale-then-refuse path as a file read.
- [x] 3.4 A budget test spanning all three producers in one request.
- [x] 3.5 Redaction tests asserting no image bytes reach logs or the transcript.

## 4. Validation

- [x] 4.1 `npm run lint:ci`
- [x] 4.2 `npm run test`
- [x] 4.3 `npm run build`
- [x] 4.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.7 `openspec validate add-onepiece-visual-tool-returns --strict`

## Status

The channel is in place, the loop resolves it, and both producers declare through it. A native tool
declares an image by putting an Artifact id under `image_artifact_id` in its result metadata; the
tool loop resolves the bytes through a hash-verified blob read, prepares them through the same
bounds and downscaling every other producer uses, and attaches the result to the reply turns.

Carrying an id rather than adding an envelope field was chosen after counting: the envelope is
constructed in more than ninety places, so a new field would have churned every native tool for a
capability two of them use. An id is also strictly safer than bytes in that position -- the
metadata is persisted on the operation record, and an id cannot smuggle base64 into it.

Every failure to attach degrades to the tool's existing result rather than failing the call: no
Artifact store wired, a text-only model, a spent per-request budget, an unreadable Artifact, or
bytes that are not a reviewed image type.

Two findings changed what shipped against what 2.2 first described. OCR cleans up its sandbox
before it builds its envelope, so the rendered pages are gone by then; what OCR declares is the
source Artifact -- exactly the page it read when the source is an image, and an unreviewed type
that degrades to text when the source is a PDF. Sealing a rendered page would improve the PDF case
and is deliberately not done here. Separately, the blob store validates content against the
declared media type when sealing, so mislabelled bytes never reach the resolver at all; the type
test now pins that stronger property rather than the one it assumed.
