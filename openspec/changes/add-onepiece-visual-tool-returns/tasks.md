## 1. Result envelope image channel

- [ ] 1.1 Add an image slot to `NativeToolResultEnvelope` alongside its text output.
- [ ] 1.2 Surface it through `execute_registered_native_tool` into the executed-call tuple `build_reply_turns` already accepts.
- [ ] 1.3 Keep base64 out of the tool output the transcript persists, and pin that with a test.
- [ ] 1.4 Land this step with the existing extended-tool tests green, before either adapter changes.

## 2. Tool surfaces

- [ ] 2.1 Return the captured image from the Browser screenshot operation alongside its existing Artifact reference.
- [ ] 2.2 Return the rendered page image from the OCR tool alongside its existing text and Artifact reference.
- [ ] 2.3 Route both through the existing image preparation so they inherit reviewed types, bounds, downscaling, and the per-request budget.
- [ ] 2.4 Degrade both to their current non-image results on a text-only model.
- [ ] 2.5 Carry the Artifact reference into the transcript for every image-returning tool.

## 3. Tests

- [ ] 3.1 Envelope tests: an image reaches the reply turns, and no base64 reaches the tool output or the persisted transcript.
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
