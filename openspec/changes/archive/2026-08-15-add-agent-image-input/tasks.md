## 1. Prerequisite

- [x] 1.1 Complete supply-chain review for the image decode/resize dependency.
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

## 4. Tool surface

- [x] 4.1 Return reviewed image types from the file tool's read operation, preserving its workspace, hidden-path, and size rules.
- [x] 4.2 Degrade the file tool to its existing non-image result on a text-only model.

## 5. Logging, transcript, and accounting

- [x] 5.1 Restrict durable logs to hash, media type, dimensions, and byte count.
- [x] 5.2 Keep image bytes out of the persisted transcript.
- [x] 5.3 Suppress character-count estimation for image-bearing requests so an image is never costed from payload length.

## 6. Tests

- [x] 6.1 Provider translation tests for both formats, plus the text-only byte-identity pin.
- [x] 6.2 Bound tests for downscaling, post-downscale refusal, and per-request image count.
- [x] 6.3 Capability-gating tests for supported, unsupported, and unknown model identifiers.
- [x] 6.4 File-tool tests covering the image read plus workspace-escape, missing-file, and content-mismatch refusals.
- [x] 6.5 Redaction and accounting tests for the log payload and the suppressed estimate.

## 7. Validation

- [x] 7.1 `npm run lint:ci`
- [x] 7.2 `npm run test`
- [x] 7.3 `npm run build`
- [x] 7.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 7.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 7.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 7.7 `openspec validate add-agent-image-input --strict`

## Scope note

Returning images from the Browser screenshot and OCR tools was in this change's first draft and is
not here. Both store output in the content-addressed Artifact store, and the agent runtime has no
port that reads Artifact bytes back by id — it holds `ArtifactPort`, which dispatches the
`artifact` *tool* and returns result envelopes. Adding that port is real work with its own
integrity and ownership rules, so it is specified separately in
`add-onepiece-visual-tool-returns` rather than left as an unchecked box here.

No frontend surface changed, so there is no Web/mock parity work for this change.
