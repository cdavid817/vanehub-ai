## Context

The survey behind this change, done before writing any of it:

- `ManagedOcrExecutionService::execute` (`ocr_execution.rs`) renders PDF pages into `render_directory`, converts each into a `PaddleOcrInferenceInput`, sends the request to the worker, and returns `PaddleOcrEngineResult`. The rendered paths exist only inside `render_directory` and inside the request it just consumed.
- `OcrNativeToolAdapter::execute_inner` calls `workspace.cleanup()` between the OCR result and the envelope it builds. Every rendered page is deleted there.
- `seal_outputs` already seals two Artifacts per published call, `ocr-result.txt` and `ocr-result.json`, both linked to the source through `source_artifact_ids`.
- `NativeToolExecutionContext` carries `call_id`, `session_id`, `generation_id`, `agent_id`, `canonical_workspace`, `deadline`, `cancelled`, and `progress`. It carries nothing about the active model, so a native tool cannot know whether an image it declares will be used.
- `render_pdf` already bounds every page it accepts: `size_bytes`, `width`, `height`, and total pixels are each checked against `OcrAdmissionLimits`, and the path must live under `render_directory`.

## Goals / Non-Goals

Goals:

- A single-page PDF OCR call returns the page pdfium rendered, not only the characters recovered from it.
- The page survives as an Artifact, addressable after the call like the text and structured result already are.
- The image travels the existing shared resolver, inheriting reviewed types, bounds, downscaling, and the per-request budget.

Non-Goals:

- Returning every page of a multi-page request. The image channel holds one Artifact id, and the per-request budget is eight images across all producers.
- Plumbing model image capability into the native tool contract. D2 explains why not here.
- Changing what an image-source OCR call returns. It already declares the page it read.
- Changing OCR admission, rendering, bounds, or the worker protocol.

## Decisions

### D1: Surface the rendered page from the execution service rather than re-reading the directory

The adapter could list `outputs/rendered` after `execute` returns and pick a file. It should not. `render_pdf` has already checked each page's dimensions, byte size, pixel count, page number, and containment; a directory listing throws that away and re-derives a weaker version of it. Widening the execution service's return value keeps the page and the checks that admitted it attached to each other.

The image-source branch renders nothing, so the returned page is `None` there — which is correct, and is why the image-source path needs no change.

### D2: Seal on render, not on capability

The tool cannot know whether the model accepts images, so it either seals a page that may go unused, or capability gets plumbed into `NativeToolExecutionContext` and every handler.

Seal unconditionally. The rendered page is evidence in exactly the sense `ocr-result.txt` and `ocr-result.json` are: it is what OCR actually read, and a user auditing a bad extraction wants to see it whether or not a model looked at it. Framing the seal as evidence rather than as image plumbing also keeps the capability check where it already lives -- the tool declares, the loop decides -- rather than splitting that decision across two layers.

The cost is bounded: one page, already limited by `rendered_page_bytes`, and content-addressed so re-reading the same page does not re-store it.

### D3: One page, and only when exactly one was rendered

The metadata channel carries a single Artifact id. For a multi-page request, any choice among the pages is arbitrary and the model has no way to tell which one it received.

Return the page only when the request rendered exactly one. This is not a consolation limit: a caller asking to look at a page passes that page, and a caller bulk-extracting text passes many and does not want an image. A multi-page call keeps today's behavior unchanged.

### D4: Seal before cleanup, declare after

`workspace.cleanup()` stays where it is. The page is read and sealed while the sandbox still exists, and the envelope is built afterwards from the resulting Artifact id. Moving cleanup later would leave the sandbox alive across the sealing failure paths.

A sealing failure degrades to today's result rather than failing the call, matching every other way an image can fail to attach.

## Risks / Trade-offs

- A single-page PDF OCR call now writes one more blob even on a text-only model. Accepted per D2; the page is evidence on its own merits.
- Multi-page requests stay text-only, so "OCR returns the page it read" is not universally true and the spec says exactly when it holds.
- Widening the execution service's return type touches its tests. That is mechanical and preferable to D1's alternative.

## Migration Plan

None. No persisted format changes, no stored data is reinterpreted, and every path that does not render exactly one page behaves as it does today.

## Open Questions

None.
