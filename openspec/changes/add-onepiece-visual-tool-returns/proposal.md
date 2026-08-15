## Why

`add-agent-image-input` gave the native Agent eyes: images reach both provider wire formats, capability is gated on reviewed metadata, and the `file` tool can return a PNG or JPEG from the workspace. What it did not do is let the two tools that actually *produce* images hand them back — the Browser screenshot and the OCR tool still return an Artifact reference and, for OCR, flattened text.

That is the original motivation left unfinished. A managed Playwright sidecar can capture a page and a checksum-verified renderer can rasterize a PDF, but the model still cannot look at either result: it can only read characters OCR recovered from it. "Is this layout broken", "did the dialog render off-screen", "what does this chart show" are exactly the questions those pixels answer and OCR destroys.

The blocker is structural, not incidental. Both tools store their output in the content-addressed Artifact store, and the agent runtime has no way to read Artifact bytes back by id — it holds `ArtifactPort`, which dispatches the `artifact` *tool* and returns result envelopes, not a byte reader.

## What Changes

- Add a read-only port that resolves an Artifact id to its bytes and media type, for Artifacts owned by the calling session, and implement it over the existing content-addressed blob store.
- Let the Browser screenshot operation return its captured image alongside the Artifact reference it already returns.
- Let the OCR tool return the rendered page image alongside its extracted text and Artifact reference.
- Route both through the existing image preparation, so they inherit the reviewed types, dimension and byte bounds, downscaling, and per-request image budget rather than getting a second path.
- Degrade both to their current non-image results when the active model does not accept images, so a model choice never turns a working tool into a failure.
- Carry images between tools and turns by Artifact id rather than by host path, and keep the transcript carrying that reference rather than bytes.

## Capabilities

### Modified Capabilities

- `agent-image-input`: Extends the set of tools that may return an image to the Browser screenshot and OCR tools, and adds Artifact-id transport for images passed between tools and turns.

## Impact

- The agent runtime gains one read-only Artifact-bytes port with a Tauri-side implementation over the existing blob store; no new persistence and no new blob format.
- The Browser and OCR extended tools gain an image on their result envelopes; their existing Artifact and text outputs are unchanged, so a text-only model sees exactly what it sees today.
- Both tools stay behind their existing feature gates and readiness predicates; this change does not make either available where it was not already.
- The per-request image budget introduced by `add-agent-image-input` now bounds three producers instead of one, which is what that budget was sized for.
- No new package dependencies are introduced.
