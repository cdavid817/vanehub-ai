## Context

The application currently initializes i18next with eager `zh-CN` and `en` resources, models the language setting as a two-value TypeScript tuple and Rust enum, and renders the Basic Configuration selector through a Chinese-versus-English conditional. Each frontend locale contains roughly 1,800 flat keys. Most date and number presentation already receives the active i18next language, but resource parity tests and project rules name only the two current locales.

Native localization is separate from React. Desktop bootstrap snapshots a language-specific `TrayCopy`, while the communications overload message reads the saved setting and independently branches between Chinese and English. These branches have different fallback behavior, and changing language while the application runs does not refresh already-created tray copy.

This change spans frontend localization, shared settings persistence, Rust validation, native lifecycle copy, Web/mock behavior, and test policy. It must preserve React-to-service boundaries, keep native behavior in Rust, retain the existing persisted string contract, and complete initial language hydration before exposing the formal application surface.

The first delivery adds `zh-TW`, `ja`, and `ko`. Spanish, French, German, and Brazilian Portuguese are expected follow-ups, so plural handling and locale registration must not remain tailored to East Asian languages.

## Goals / Non-Goals

**Goals:**

- Support `zh-CN`, `en`, `zh-TW`, `ja`, and `ko` as complete application locales in desktop and Web/mock runtimes.
- Make one frontend registry the source of locale metadata, resource loading, ordering, and selector identity.
- Keep frontend and Rust allowlists explicitly aligned and verify their parity.
- Load optional locale resources without adding all translations to the initial Web bundle.
- Apply saved language before first visible application render and preserve immediate in-app switching.
- Give native tray, close notice, and communications overload copy the same locale resolution and fallback semantics.
- Make count-bearing messages ready for i18next plural categories and verify translation structure automatically.
- Preserve the current default and fallback locale, stored settings, and runtime adapter boundaries.

**Non-Goals:**

- Automatic operating-system locale detection or a “follow system” setting.
- Runtime download of translations, remote translation services, or user-installed language packs.
- Arabic/right-to-left layout, Russian plural coverage, or the second batch of European locales.
- Translating README files, user guides, developer documentation, provider output, CLI output, user content, or stable product identifiers.
- Replacing i18next/react-i18next, changing state management, or adding direct Tauri calls to React components.

## Decisions

### Use canonical BCP 47 locale ids and a typed frontend registry

The supported ids are `zh-CN`, `en`, `zh-TW`, `ja`, and `ko`. A frontend locale registry will associate each id with its selector translation key, text direction, deterministic ordering, and resource loader. `AppLanguage`, settings normalization, i18next resource resolution, and the language selector will consume this registry instead of maintaining binary checks or separate arrays.

The selector will obtain labels from locale resources rather than hard-coded React literals. Each locale will present recognizable language names, including native-script names, so users can recover even after selecting an unfamiliar locale.

Alternative considered: use `Intl.DisplayNames` as the only source for selector labels. Rejected because browser support and returned wording are not controlled by the product, and the current localization policy requires user-visible product copy to live in translation resources.

### Keep `zh-CN` as the default and deterministic fallback

The existing default and `fallbackLng` remain `zh-CN` to avoid changing first-run behavior or silently changing restored installations. An unknown or unavailable locale is rejected at the persistence boundary; if an already-selected optional resource fails to load, startup applies `zh-CN`, exposes a localized settings error, and still renders the application.

Alternative considered: change the fallback to English because English is common for developer tools. Rejected for this change because it is a separate product-default decision and is not required to add locales safely.

### Lazy-load optional frontend resources through local Vite chunks

`zh-CN` remains available synchronously for deterministic fallback. Other locale resources are loaded by explicit registry loaders from bundled local files; no network translation endpoint is introduced. The SettingsProvider waits for the selected resource and `changeLanguage` during initial hydration, preserving the current no-language-flash contract. Later switches await the resource before committing the visible locale.

Tests may preload requested resources through the same public localization helper. This avoids tests depending on implementation-only imports while keeping startup behavior representative.

Alternative considered: eagerly import all five resources into the main bundle. Rejected because each current resource is about 100 KB before bundling and the planned second batch would make startup cost grow linearly for translations that a session does not use.

### Preserve the settings service contract and persist locale ids as strings

`applicationLanguage` remains a string-valued field in the shared settings DTO. TypeScript normalization and the Rust `ApplicationLanguage` domain type expand their accepted values, while Tauri and Web/mock adapters keep their existing request and response shapes. SQLite continues storing the canonical locale id, so no schema migration is required.

The frontend registry and Rust enum remain separate compile-time representations because the two runtimes have different build systems. Contract tests will assert the same ordered supported-id set at their boundary rather than introducing generated code or runtime file parsing into Rust bootstrap.

Alternative considered: make Rust consume the frontend JSON registry. Rejected because it couples native bootstrap to frontend asset layout and weakens native domain validation.

### Centralize native copy resolution behind the desktop settings side-effect boundary

Rust will resolve native copy from `ApplicationLanguage` rather than raw string predicates. One native localization catalog will provide tray actions, close-to-tray notices, and communications overload copy for every supported locale with the same `zh-CN` fallback.

Saving `applicationLanguage` will notify a native locale side-effect port owned by the desktop settings/application layer. The Tauri lifecycle implementation will retain handles needed to update existing tray menu item text and future close notices without restarting the process. Communications copy may continue resolving against current settings at message time, but it will use the same catalog.

The Web/mock adapter persists and applies the same locale ids to React but does not claim tray or background-message native behavior. React components continue using the settings provider; no new component-level `invoke()` is added.

Alternative considered: update native labels only after restart. Rejected because the language selector currently applies immediately and leaving persistent native controls in the previous language produces an inconsistent setting contract.

### Generalize resource integrity checks and plural conventions

English will serve as the canonical key and interpolation-token set for automated comparison, while `zh-CN` remains the runtime fallback. Tests will discover registered locales and require:

- identical keys across every registered resource;
- no duplicate raw JSON keys;
- identical interpolation variable names for each key;
- registered resource files and registry entries to match exactly;
- valid i18next v4 plural suffixes for count-sensitive messages; and
- representative translations and language switching for each supported locale.

Count-sensitive call sites will pass a numeric `count` so i18next can select locale-specific plural categories. Locale-sensitive numeric presentation will use i18next/Intl formatting rather than converting `count` to a formatted string before plural selection. Every registered locale will provide explicit i18next v4 `_one` and `_other` resource pairs for count-sensitive keys. Languages without grammatical-number distinctions may use identical values for the pair, but the uniform shape keeps registry-wide key parity deterministic and leaves future locale additions reviewable.

Alternative considered: retain parenthetical English forms such as “item(s)”. Rejected because that cannot express correct French, German, Portuguese, Russian, or Arabic grammar and would preserve a known blocker for follow-up locales.

### Treat application localization and documentation localization as separate products

The runtime locale catalog does not imply matching README or user-guide coverage. Documentation retains its existing canonical-source and supported-language rules. This avoids multiplying screenshot, navigation, and review obligations inside a UI localization change.

## Risks / Trade-offs

- [Three new resources require more than 5,000 translated entries and may contain semantically weak translations] → Use an English glossary/canonical-key review, preserve placeholders mechanically, review high-risk actions and errors by domain, and require representative rendered-page review in each locale.
- [Traditional Chinese can drift into mechanical character conversion] → Review terminology, regional phrasing, and destructive-action wording independently instead of treating `zh-TW` as generated `zh-CN` output.
- [Longer translations can overflow controls] → Add Playwright smoke coverage at desktop and narrow viewports and inspect settings, dialogs, navigation, notifications, and chat controls in the longest representative locale.
- [Lazy loading can reintroduce startup language flash or a blank screen] → Keep `zh-CN` locally available, await resource loading inside settings hydration, and test delayed and failed loaders.
- [Frontend and Rust supported-locale sets can drift] → Add boundary contract tests that compare the serialized Rust-supported ids with the frontend registry or an equivalent checked fixture.
- [Updating an existing tray menu may fail on a platform-specific Tauri path] → Keep the previous valid labels, report a redacted warning through unified logging, and allow the language setting and React UI to remain applied.
- [The completed but unarchived Basic Configuration information-architecture change touches the same selector] → Implement against its current result and verify/archive that change before merging this implementation when practical.
- [A new locale saved by this version is unknown to an older downgraded binary] → Older versions may fall back to their existing default; this change does not rewrite stored values during downgrade.

## Migration Plan

1. Add generalized localization tests and the typed frontend registry while retaining the two existing locales and behavior.
2. Expand Rust language validation and introduce the shared native localization catalog and runtime update side effect.
3. Add and review `zh-TW`, `ja`, and `ko` resources, then enable them in the registry and language selector.
4. Migrate count-sensitive messages to numeric plural selection and verify locale-sensitive number/date formatting.
5. Run focused unit, Rust, Playwright, build, and strict OpenSpec validation before release.

No SQLite migration is required. Rollback removes the new locale entries and loaders; installations that persisted a new locale will be handled by the older runtime's existing invalid-value fallback behavior.

## Open Questions

None. The first-batch locale set, fallback, runtime boundaries, and documentation exclusion are fixed by this proposal.
