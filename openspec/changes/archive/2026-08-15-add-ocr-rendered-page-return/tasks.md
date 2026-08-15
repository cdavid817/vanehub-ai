## 1. Keep the rendered page

- [x] 1.1 Widen `ManagedOcrExecutionService::execute` to return the rendered page alongside the engine result, keeping the bounds `render_pdf` already applied attached to it.
- [x] 1.2 Return no page for an image source, which renders nothing.
- [x] 1.3 Return no page when a request rendered more than one, so the single-page condition is decided in one place.

## 2. Seal and declare

- [x] 2.1 Seal the rendered page as an Artifact before `workspace.cleanup()`, linked to the source through `source_artifact_ids` like the existing outputs.
- [x] 2.2 Declare the sealed page under the existing image metadata key, replacing the source Artifact for this case only.
- [x] 2.3 Leave the image-source declaration as it is.
- [x] 2.4 Degrade to today's text result when sealing fails, rather than failing the call.

## 3. Tests

- [x] 3.1 A single-page PDF OCR call declares the rendered page, not the PDF source.
- [x] 3.2 A multi-page PDF OCR call declares no image and returns its text unchanged.
- [x] 3.3 An image-source call still declares the source page.
- [x] 3.4 The declared page is a reviewed image type that resolves through the shared path, and the sandbox is still cleaned up.
- [x] 3.5 A sealing failure degrades to the text result.

## 4. Validation

- [x] 4.1 `npm run lint:ci`
- [x] 4.2 `npm run test`
- [x] 4.3 `npm run build`
- [x] 4.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 4.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 4.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 4.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 4.8 `openspec validate add-ocr-rendered-page-return --strict`
- [x] 4.9 `openspec validate --specs --strict`

## Status

A rasterized single-page OCR call now returns the page pdfium drew. `execute` returns an
`OcrExecutionOutcome` carrying the engine result plus the lone rendered page; the adapter seals that
page as an `image/png` Artifact linked to the source, before `workspace.cleanup()` deletes it, and
declares it under the existing image metadata key. Everything else -- image sources, multi-page
requests, unreviewed source types, and a page that could not be retained -- declares the source and
degrades to text exactly as before.

Two things were easier than the design assumed. `RenderedOcrPage` already carried the page number,
path, and dimensions, so widening the return type cost a struct and no new plumbing; and
`render_pdf` was already the single place every page bound is checked, so returning validated pages
from it rather than inference inputs kept those checks attached to the page.

The adapter's PDF path is covered end to end rather than only at the seam. The renderer is
constructed inside `execute_inner` and is not injectable, but it launches through the same
`SandboxProcessBackend` the OCR worker does, so one scripted backend dispatching on the requested
result file drives inspect, render, and OCR in a single test -- writing a real PNG, because
everything downstream verifies bytes rather than trusting the file name.
