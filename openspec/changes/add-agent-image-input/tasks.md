## 1. Prerequisite

- [x] 1.1 Complete supply-chain review for the image decode/resize dependency, or scope the first delivery to images already within bounds.
  - `image 0.25` with `default-features = false, features = ["png", "jpeg"]`. Adds 6 crates: `image`, `zune-jpeg`, `zune-core`, `byteorder-lite`, `moxcms`, `pxfm`. All pure Rust, no C toolchain. PNG reuses the `png` crate already in the tree. The CI `dependency-review` gate (fail-on-severity: moderate) is the enforcing check on the pull request.

## 2. Provider and representation

- [x] 2.1 Add one internal image representation carrying bytes, media type, and dimensions.
- [x] 2.2 Translate it into the Anthropic image content block and the OpenAI-compatible image shape at the existing content-translation site.
- [x] 2.3 Pin that a request carrying no image produces a byte-identical body to before.
- [x] 2.4 Add reviewed image media types and reject a declared type that disagrees with decoded content.

## 3. Capability gating and bounds

- [x] 3.1 Derive image capability from reviewed model metadata, treating unknown identifiers as unsupported.
- [x] 3.2 Enforce dimension, byte, and per-request image bounds, downscaling for dimensions and refusing for the rest.
- [x] 3.3 Report downscaling in the result that carries the image.

## 4. Tool surfaces

- [x] 4.1 Return reviewed image types from the file tool's read operation, preserving its workspace, hidden-path, and size rules.
- [ ] 4.2 Return the captured image from the Browser screenshot operation alongside its Artifact reference.
- [ ] 4.3 Return the rendered page image from the OCR tool alongside its extracted text.
- [x] 4.4 Degrade every image-capable tool to its existing non-image result on a text-only model.
  - Delivered for the file tool: on a text-only model an image read falls through to the ordinary text path and returns its existing binary-content refusal. Re-check when 4.2/4.3 land.

## 5. Logging, transcript, and accounting

- [x] 5.1 Restrict durable logs to hash, media type, dimensions, and byte count.
- [ ] 5.2 Persist an Artifact reference in the transcript instead of embedding image bytes.
  - The transcript already carries only a summary line, never bytes, so the harmful half is closed. The Artifact reference itself is still outstanding and belongs with 4.2/4.3, which are what make tool-to-tool image transfer real.
- [ ] 5.3 Attribute provider-reported usage for image requests and suppress character-count estimation for them.

## 6. Tests

- [x] 6.1 Provider translation tests for both formats, plus the text-only byte-identity pin.
- [x] 6.2 Bound tests for downscaling, post-downscale refusal, and per-request image count.
- [x] 6.3 Capability-gating tests for supported, unsupported, and unknown model identifiers.
- [x] 6.4 Tool tests for file read, including workspace-escape, missing-file, and content-mismatch refusals. Screenshot and OCR coverage lands with 4.2/4.3.
- [x] 6.5 Redaction: `log_image_attachment` emits hash, media type, dimensions, and byte count only.
- [ ] 6.6 Web/mock parity tests.

## 7. Validation

- [x] 7.1 `npm run lint:ci`
- [x] 7.2 `npm run test`
- [x] 7.3 `npm run build`
- [x] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 7.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 7.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 7.7 `openspec validate add-agent-image-input --strict`

## Status

Sections 1–3 and the file-tool surface are complete and verified. The change stays open: the
Browser screenshot and OCR image returns (4.2/4.3), the Artifact reference in the transcript
(5.2), image token accounting (5.3), and Web/mock parity (6.6) are not implemented, so this must
not be archived — archiving would publish spec requirements the code does not yet satisfy.
