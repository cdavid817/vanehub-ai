## Why

`add-onepiece-visual-tool-returns` gave the OCR tool an image channel and deliberately left one case open: a PDF source still comes back as text only. What OCR declares today is its *source* Artifact, which is exactly the page it read when the source is an image — and a PDF when it is not. A PDF is not a reviewed image type, so the loop finds nothing to attach and the call degrades to characters.

That leaves the more valuable half of OCR unlit. Reading an image the user already has is the case where the model could have looked at the file directly. Rasterizing a PDF page is the case where it *cannot* — pdfium is the only thing in the system that turns that page into pixels, and today those pixels are produced, read by the OCR worker, and deleted. "Is this table misaligned", "what does this chart show", "did the signature block render" are answerable from the page and not from the text recovered from it.

The blocker is that nothing keeps the page. `ManagedOcrExecutionService::execute` renders pages into the sandbox, hands their paths to the worker as inference inputs, and returns only `PaddleOcrEngineResult` — the paths are dropped. `execute_ocr` then calls `workspace.cleanup()` before it builds its envelope, so by the time there is an envelope to declare an image on, the page is gone.

## What Changes

- Surface the rendered page from the OCR execution service instead of discarding it after the worker consumes it.
- Seal the rendered page as an Artifact before the sandbox is cleaned up, so the page OCR read survives as evidence in the same sense the extracted text and structured result already do.
- Declare that sealed page as the OCR result's image, so a PDF source returns the page rather than only its characters.
- Limit this to a single rendered page. The image channel carries one Artifact id, and choosing one page out of a multi-page request would be arbitrary; a multi-page call keeps returning text exactly as it does today.
- Leave the image-source case untouched: an image source already declares the page it read.

## Capabilities

### Modified Capabilities

- `agent-image-input`: Narrows what the OCR tool returns for a PDF source from "no image" to the rendered page, and states the single-page condition under which it does so.

## Impact

- The OCR execution service's return type gains the rendered page; its callers are the OCR adapter and its tests.
- A single-page PDF OCR call seals one additional Artifact. The tool cannot know whether the model accepts images — no capability signal reaches `NativeToolExecutionContext` — so this cost is paid whenever a single page is rendered, not only when an image will be attached. Design D2 records why that is acceptable rather than worth plumbing capability down.
- Image-source OCR calls, multi-page PDF calls, failed calls, and cancelled calls are unchanged.
- The image inherits the existing bounds, downscaling, and per-request budget through the shared resolver; this change adds no second image path.
- No new package dependencies, no new port, no new persistence format.
