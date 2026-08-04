## Why

Assistant replies currently render basic Markdown and Mermaid, but common AI output such as tables, mathematical expressions, highlighted source code, and remote images is incomplete or blocked in the desktop runtime. A consistent rich-media mode is needed so technical answers remain readable and useful in both Tauri and Web environments.

## What Changes

- Extend chat Markdown rendering with GitHub Flavored Markdown, mathematical notation, and syntax-highlighted fenced code blocks.
- Render safe HTTPS and data images responsively with lazy loading, failure fallback, and an accessible enlarged preview.
- Preserve Mermaid rendering and improve its failure fallback so the source remains available.
- Keep raw provider HTML disabled and constrain all rich media to the message layout.
- Apply the same React renderer in desktop and Web runtimes; update the Tauri image content-security policy only for explicitly supported image protocols.
- Add automated coverage for supported markup, unsafe image URLs, image preview behavior, and Mermaid fallback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `chat-experience`: Expand assistant reply rendering requirements from basic Markdown and Mermaid to safe, responsive rich-media content in desktop and Web runtimes.

## Impact

- Frontend chat rendering components and localized chat labels.
- Frontend Markdown dependencies and global styles for math and code highlighting.
- Tauri CSP `img-src` policy for HTTPS images.
- No new service methods, Tauri commands, database schema, or frontend/backend boundary changes.
