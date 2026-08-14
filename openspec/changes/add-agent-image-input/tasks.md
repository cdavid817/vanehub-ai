## 1. Prerequisite

- [ ] 1.1 Complete supply-chain review for the image decode/resize dependency, or scope the first delivery to images already within bounds.

## 2. Provider and representation

- [ ] 2.1 Add one internal image representation carrying bytes, media type, and dimensions.
- [ ] 2.2 Translate it into the Anthropic image content block and the OpenAI-compatible image shape at the existing content-translation site.
- [ ] 2.3 Pin that a request carrying no image produces a byte-identical body to before.
- [ ] 2.4 Add reviewed image media types and reject a declared type that disagrees with decoded content.

## 3. Capability gating and bounds

- [ ] 3.1 Derive image capability from reviewed model metadata, treating unknown identifiers as unsupported.
- [ ] 3.2 Enforce dimension, byte, and per-request image bounds, downscaling for dimensions and refusing for the rest.
- [ ] 3.3 Report downscaling in the result that carries the image.

## 4. Tool surfaces

- [ ] 4.1 Return reviewed image types from the file tool's read operation, preserving its workspace, hidden-path, and size rules.
- [ ] 4.2 Return the captured image from the Browser screenshot operation alongside its Artifact reference.
- [ ] 4.3 Return the rendered page image from the OCR tool alongside its extracted text.
- [ ] 4.4 Degrade every image-capable tool to its existing non-image result on a text-only model.

## 5. Logging, transcript, and accounting

- [ ] 5.1 Restrict durable logs to hash, media type, dimensions, and byte count.
- [ ] 5.2 Persist an Artifact reference in the transcript instead of embedding image bytes.
- [ ] 5.3 Attribute provider-reported usage for image requests and suppress character-count estimation for them.

## 6. Tests

- [ ] 6.1 Provider translation tests for both formats, plus the text-only byte-identity pin.
- [ ] 6.2 Bound tests for downscaling, post-downscale refusal, and per-request image count.
- [ ] 6.3 Capability-gating tests for supported, unsupported, and unknown model identifiers.
- [ ] 6.4 Tool tests for file read, screenshot, and OCR image returns, including the text-only degradation path.
- [ ] 6.5 Redaction tests asserting no image bytes reach logs or the persisted transcript.
- [ ] 6.6 Web/mock parity tests.

## 7. Validation

- [ ] 7.1 `npm run lint:ci`
- [ ] 7.2 `npm run test`
- [ ] 7.3 `npm run build`
- [ ] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 7.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 7.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 7.7 `openspec validate add-agent-image-input --strict`
