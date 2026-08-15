## 1. Result envelope image channel

- [x] 1.1 Carry the image as an Artifact id in the envelope's existing metadata rather than adding a field, keeping the change additive across ninety-odd construction sites.
- [x] 1.2 Surface it through `execute_registered_native_tool` into the executed-call tuple `build_reply_turns` already accepts.
- [x] 1.3 Keep base64 out of the tool output the transcript persists, and pin that with a test.
- [x] 1.4 Land this step with the existing extended-tool tests green, before either adapter changes.

## 2. Tool surfaces

- [ ] 2.1 Return the captured image from the Browser screenshot operation alongside its existing Artifact reference.
- [ ] 2.2 Return the rendered page image from the OCR tool alongside its existing text and Artifact reference.
- [ ] 2.3 Route both through the existing image preparation so they inherit reviewed types, bounds, downscaling, and the per-request budget.
- [ ] 2.4 Degrade both to their current non-image results on a text-only model.
- [ ] 2.5 Carry the Artifact reference into the transcript for every image-returning tool.

## 3. Tests

- [x] 3.1 Envelope tests: an image reaches the reply turns, and no base64 reaches the tool output or the persisted transcript.
- [ ] 3.2 Screenshot and OCR image-return tests, including the text-only degradation path.
- [ ] 3.3 A bound test proving a produced image goes through the same downscale-then-refuse path as a file read.
- [ ] 3.4 A budget test spanning all three producers in one request.
- [ ] 3.5 Redaction tests asserting no image bytes reach logs or the transcript.

## 4. Validation

- [ ] 4.1 `npm run lint:ci`
- [ ] 4.2 `npm run test`
- [ ] 4.3 `npm run build`
- [ ] 4.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 4.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 4.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 4.7 `openspec validate add-onepiece-visual-tool-returns --strict`

## Status

The channel is in place and the loop resolves it. A native tool declares an image by putting the
Artifact id it just sealed under `image_artifact_id` in its result metadata; the tool loop reads
that id, resolves the bytes through a hash-verified blob read, prepares the image through the same
bounds and downscaling every other producer uses, and attaches it to the reply turns.

Carrying an id rather than adding an envelope field was chosen after counting: the envelope is
constructed in more than ninety places, so a new field would have churned every native tool for a
capability two of them use. An id is also strictly safer than bytes in that position -- the
metadata is persisted on the operation record, and an id cannot smuggle base64 into it.

Every failure to attach degrades to the tool's existing result rather than failing the call: no
Artifact store wired, a text-only model, a spent per-request budget, an unreadable Artifact, or
bytes that are not a reviewed image type.

Remaining: the two producers (2.1, 2.2) still need to set the metadata key, plus their tests
(3.2-3.5). Neither needs a new read -- the screenshot adapter already holds its bytes and seals
them, and OCR renders into its own sandbox.
