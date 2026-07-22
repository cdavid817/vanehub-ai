## Implementation

- [x] Rework the About settings page into a responsive horizontal layout.
- [x] Merge product identity, metadata, update controls, and external links into one software details panel.
- [x] Merge changelog and product positioning into one related information panel.
- [x] Keep runtime/agent and local CLI environment content removed from the About page.
- [x] Update regression coverage for the About page.

## Verification

- [x] Run `npx tsc --noEmit`.
- [x] Run `npx vitest run src/settings/pages/about-page.test.tsx src/i18n/i18n-resource-parity.test.ts`.
- [x] Run `npm run build`.
- [x] Run `openspec validate settings-about-page-layout --strict`.
