## Context

`SettingsProvider` initializes React state with `defaultAppSettings`, but it does not apply those values to the document during the initial render. Its children are rendered immediately, while the persisted settings read runs later in an effect through the active runtime adapter. The browser therefore paints the application with its default 16px root size and base theme before the provider writes the configured root font size and `data-theme`, causing a full rem-based layout reflow.

Both the main application and floating-assistant surface mount the same provider. Desktop settings arrive asynchronously through the Tauri adapter and SQLite-backed command; Web/mock settings arrive through the interface-compatible Web adapter.

## Goals / Non-Goals

**Goals:**

- Prevent settings-dependent application children from becoming visible before initial settings are normalized and applied.
- Preserve successful restoration for both desktop and Web/mock runtimes.
- Apply shared defaults before rendering when the initial settings read fails.
- Keep the existing settings service and runtime adapter boundaries unchanged.
- Cover the ordering contract with focused frontend tests.

**Non-Goals:**

- Change settings persistence, supported values, SQLite schema, or Tauri commands.
- Add a general-purpose splash-screen framework.
- Change optimistic settings updates after startup.
- Alter the fixed xterm terminal font size.

## Decisions

1. Gate provider children on initial settings hydration.

   `SettingsProvider` will continue mounting immediately so its effects can load settings, but it will return no formal application surface while the initial load is pending. After the selected persisted or fallback settings have been applied, the provider will clear its loading state and expose its children.

   This keeps ownership of settings side effects in the provider, applies equally to the main and floating surfaces, and avoids moving runtime-specific knowledge into `main.tsx`.

   Alternative considered: set `html { font-size: 14px }` in static CSS. That only masks the issue for the 14px default and still reflows users configured for 12px or 18px.

   Alternative considered: replace `useEffect` with `useLayoutEffect`. The settings read remains asynchronous, so the browser can still paint before its promise resolves.

2. Complete document side effects before releasing the hydration gate.

   The provider will apply the root font size and theme synchronously and await the bundled i18next language change during initial hydration. Only after this work completes will settings state become visible to child consumers.

   Alternative considered: render children with default context state while only hiding them with CSS. That still mounts settings-dependent components with incorrect values and can start avoidable queries or terminal work before hydration.

3. Treat fallback application as successful hydration with an error.

   If `getSettings` fails, the provider will apply `defaultAppSettings`, retain the error message, and then render. This preserves fail-open startup behavior without reintroducing an unconfigured first paint.

4. Test the visibility and ordering contract at the provider boundary.

   A deferred settings-service result will verify that children do not render while hydration is pending and that their first render observes the configured document settings. A rejected result will verify default application and error exposure.

## Risks / Trade-offs

- [A slow native settings read leaves the React root temporarily empty] → Keep the gate limited to the initial settings request; do not wait for independent Node.js discovery or other startup data.
- [Server-render-only tests cannot run effects and would render an empty provider] → Exercise settings pages in the existing jsdom client environment and wait for hydration, matching the supported Vite/Tauri runtime.
- [Awaiting language synchronization could marginally extend startup] → Resources are bundled locally, and correctness before first visibility is preferable to a language flash.
- [A future settings read that never settles would keep the surface hidden] → Preserve adapter error propagation and cover successful and rejected completion; timeout policy remains an adapter/runtime concern.

## Migration Plan

1. Add provider hydration regression tests.
2. Gate children until persisted or fallback settings are applied.
3. Run focused tests, the complete frontend/Rust validation suite, and strict OpenSpec validation.

Rollback is a frontend-only revert of the provider gate and tests; no stored data or native schema migration is involved.

## Open Questions

None.
