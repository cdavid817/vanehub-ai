## Why

OnePiece can drive a browser and take screenshots (`browser`'s `screenshot` operation), render PDF pages, and store both as Artifacts — but it cannot look at any of them. The provider request builders in `anthropic_provider.rs` and `openai_compatible_provider.rs` emit text-only content, so every visual result has to be laundered through OCR into a flat string. That is the wrong tool for the job: OCR recovers characters, not layout, not colour, not whether the button is actually overlapping the heading.

This leaves the most expensive capabilities already shipped — a managed Playwright sidecar, a checksum-verified PDF renderer, a content-addressed Artifact store — delivering a fraction of what they could. It also blocks the ordinary case of a user pasting a screenshot of a broken UI into the chat.

## What Changes

- Add image content blocks to both provider wire formats, so an image can be sent as part of a user turn or a tool result.
- Let the `file` tool's read operation return an image as an image block instead of refusing it as binary content, for reviewed image types only.
- Add model-capability gating: images are offered only when the active Profile's model is known to accept them, and a session whose model does not is told so rather than failing at the provider.
- Bound every image path by declared maximum dimensions, encoded bytes, and images per request, downscaling or refusing rather than sending an unbounded payload.
- Keep image bytes out of durable logs and out of the persisted transcript.

## Capabilities

### New Capabilities

- `agent-image-input`: Defines image content blocks, the reviewed image types, capability gating, bounds, redaction, token accounting, and the file tool's image read. Returning images from the Browser screenshot and OCR tools needs a way to read Artifact bytes back by id, which the agent runtime has no port for; that lands in `add-onepiece-visual-tool-returns`.

### Modified Capabilities

- `agent-provider-runtime`: Adds image content-block translation for both interface formats.

## Impact

- The Rust runtime gains image encoding, downscaling, and a per-request image budget; both provider modules gain a content-block shape they do not have today.
- Token accounting must attribute image tokens, which are reported by the provider but not derivable from character counts.
- The Artifact store becomes the transport for image bytes between tools and turns, so no new persistence is introduced.
- Redaction rules extend to image payloads: durable logs carry hashes, dimensions, and byte counts only.
- No new package dependencies beyond an image decode/resize crate, which requires a supply-chain review before adoption.
