# Agent icon provenance

Brand marks for the seven CLI Agents added by `extend-cli-providers-with-acp`, fetched on
2026-09-07. Each file identifies the vendor's program in the Agent selector, session list, and
CLI management page, the way the Claude, OpenAI, Gemini, OpenCode, and Antigravity marks already
do. Rasters were downscaled to 64×64 RGBA PNG with Pillow (LANCZOS); vectors are unmodified.

| File | Source | Notes |
| --- | --- | --- |
| `qwen-code.png` | `https://qwenlm.github.io/qwen-code-docs/favicon.png` (406×406, the Qwen Code documentation site's own icon) | Downscaled |
| `kimi-cli.png` | `https://www.kimi.com/pwa-192.png` (the Kimi web app's PWA icon, linked from kimi.com) | Downscaled |
| `qoder-cli.png` | `https://img.alicdn.com/imgextra/i4/O1CN01QkSxiCocd3D0prc8_!!6000000008124-2-tps-412-412.png` (qoder.com's `<link rel="icon">`) | Downscaled |
| `codebuddy-code.svg` | `dist/web-ui/pwa-icon.svg` inside the installed npm package `@tencent-ai/codebuddy-code@2.146.0` (the web UI's PWA icon; the vendor's site refuses scripted access) | Unmodified |
| `cursor-agent-cli.svg` | `https://cursor.com/favicon.svg` | Unmodified |
| `iflow-cli.png` | `https://img.alicdn.com/imgextra/i4/O1CN01yBfg3x1iNi4YggwIt_!!6000000004401-2-tps-72-72.png` (iflow.cn's `<link rel="icon">`) | 72×72 source, downscaled to 64 |
| GitHub Copilot | Inlined in `src/components/agent-brand-icon.tsx` from Primer Octicons `copilot-24.svg` (`https://github.com/primer/octicons`, MIT) | Rendered with `currentColor` |

The Octicon is MIT-licensed. The other marks are the vendors' own published favicons or
application icons; they may be trademarks of their respective owners, and their inclusion
identifies the corresponding program without implying endorsement, sponsorship, or affiliation
with VaneHub AI. Replace a file from the same source if a vendor updates its mark.
