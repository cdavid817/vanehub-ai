## 1. Markdown rendering foundation

- [x] 1.1 Add GFM, math, KaTeX, and syntax-highlighting dependencies.
- [x] 1.2 Extract a shared safe rich Markdown renderer and apply it to chat messages and Markdown-bearing Rich Blocks.
- [x] 1.3 Add responsive GFM, KaTeX, and highlighted-code styling for light and dark themes.

## 2. Images and Mermaid

- [x] 2.1 Add a shared safe image renderer with URL validation, lazy loading, no-referrer behavior, and localized failure fallback.
- [x] 2.2 Add an accessible bounded image preview with close button, backdrop close, and Escape handling.
- [x] 2.3 Reuse the image renderer for Markdown and media-gallery images and permit HTTPS images in the desktop CSP.
- [x] 2.4 Preserve Mermaid source text in the localized render-failure fallback.

## 3. Verification

- [x] 3.1 Add component tests for GFM, math, highlighted code, safe and unsafe images, image preview, and Mermaid failure source.
- [x] 3.2 Run frontend lint, tests, and production build.
- [x] 3.3 Run Rust tests/check/clippy and strict OpenSpec validation.
