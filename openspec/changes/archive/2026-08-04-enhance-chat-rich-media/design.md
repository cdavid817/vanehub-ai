## Context

Chat messages currently use `react-markdown`, a custom Mermaid code renderer, and separate structured Rich Block components. This provides basic Markdown and durable structured blocks, but it does not parse GFM or math, does not highlight source code, and the desktop CSP prevents HTTPS images. The implementation must remain a shared React concern so desktop and Web render the same persisted message content.

## Goals / Non-Goals

**Goals:**

- Provide one safe Markdown rendering pipeline for standard assistant replies.
- Support GFM, Mermaid, KaTeX math, syntax-highlighted code, and responsive images.
- Let users inspect images without allowing provider content to escape the chat layout.
- Keep rendering usable during streaming and retain localized failure fallbacks.
- Preserve the existing Rich Block contract and runtime service boundaries.

**Non-Goals:**

- Allow arbitrary raw HTML or JavaScript in Markdown.
- Download or persist remote media into SQLite.
- Add image generation, file upload, or provider-specific media APIs.
- Enable actions in read-only interactive Rich Blocks.

## Decisions

### Use React Markdown plugins for standards-based content

The shared renderer will add `remark-gfm`, `remark-math`, `rehype-katex`, and `rehype-highlight`. This keeps Markdown as the persisted source of truth and composes with the existing Mermaid code override. Building custom parsers was rejected because it would duplicate mature parsers and create inconsistent edge-case behavior.

### Keep raw HTML disabled

The renderer will not add `rehype-raw`. Provider HTML therefore remains text rather than entering the application document. Mermaid continues using its strict security level. Structured `html_widget` content remains isolated in the existing sandboxed iframe.

### Render images through a constrained component

Markdown and media-gallery images will share a reusable image component. It will accept only `https:`, `data:image/`, and application-owned relative or asset URLs, lazy-load remote resources, avoid referrer leakage, show a localized error state, and open an accessible bounded preview. Plain HTTP and active/non-image schemes will be rejected. The desktop CSP will add `https:` to `img-src`; Web mode needs no runtime-specific branch.

### Keep Mermaid lazy and expose failure source

Mermaid remains dynamically imported to protect the initial bundle. On parse failure, the renderer will show localized feedback and the original diagram source in a bounded code block, matching the existing specification.

### Use CSS classes rather than inline styles

KaTeX supplies its required package stylesheet. Highlight output will use semantic `hljs` classes styled through the project global Tailwind stylesheet so code remains readable in both themes without a second UI library.

## Risks / Trade-offs

- [Remote images can disclose the user's IP to the image host] → Load only explicit HTTPS URLs from reply content, use `referrerPolicy="no-referrer"`, and do not prefetch.
- [Plugin parsing on every streamed token can be expensive] → Preserve memoized message rows and keep Mermaid dynamically imported; only the active streaming row reparses.
- [Very large images or diagrams can disrupt layout] → Bound preview dimensions and use scroll/contain behavior.
- [KaTeX and highlighting increase frontend bundle size] → Keep Mermaid lazy, verify chunk budgets, and use renderer plugins instead of a full editor framework.
- [Some programming languages may not be detected] → Fall back to a readable unhighlighted code block.

## Migration Plan

No data migration is required because persisted message content remains Markdown text and existing Rich Blocks are unchanged. Deploy the frontend dependencies, shared renderer, and Tauri CSP update together. Rollback consists of reverting the renderer plugins and CSP entry; stored messages remain compatible as plain Markdown.

## Open Questions

None for the initial implementation.
