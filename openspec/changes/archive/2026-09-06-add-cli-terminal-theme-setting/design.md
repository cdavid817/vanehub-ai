## Context

`terminal-theme.ts` builds the xterm theme by reading `--terminal-*` custom properties from `document.documentElement`. Those properties are declared once on `:root` as a dark set and shared by the Agent terminal (`.ucd-agent-terminal`) and the ordinary Shell (`.ucd-shell-terminal`). Both components already own a `MutationObserver` on `data-theme` that reassigns `terminal.options.theme`, so a hot-update path exists; what is missing is a second axis that only the Agent terminal follows.

## Goals / Non-Goals

**Goals:**

- One setting, two complete palettes, applied to every single-Agent CLI terminal that is open, hidden, or created later.
- Hot update through the existing observer path with no xterm recreation, no reconnect, no buffer loss.
- CSS remains the single source of the colors; TypeScript keeps only the dark fallbacks it already had.

**Non-Goals:**

- Follow-system, per-session override, automatic detection, or a toolbar shortcut.
- Repainting Shell, remote terminals, native Agent chat, or any other surface.
- Rewriting the CLI's own 256-color or truecolor output, `filter: invert()`, ANSI class overrides, or touching the user's CLI configuration.

## Decisions

### 1. The attribute lives on the root, the variables live on the container

`applySettings` sets `document.documentElement.dataset.cliTerminalTheme`. The light palette is declared as `:root[data-cli-terminal-theme="light"] .ucd-agent-terminal { --terminal-…: … }`, so the override only resolves inside Agent terminal containers. The Shell keeps reading the root defaults. Putting the attribute on the root rather than on each container means one observer target and no per-component plumbing; scoping the variables by selector is what keeps the Shell out.

### 2. `createTerminalTheme(element?)`

The function takes an optional element and reads computed style from it, defaulting to the document root. The Agent terminal passes its host element; `shell-surface.tsx` keeps its no-argument call and behavior. Because the variables are resolved on the container, xterm and the CSS frame always agree; the root cannot be light while xterm reads dark.

### 3. Hot update rides the existing observer

The Agent terminal's `MutationObserver` filter gains `data-cli-terminal-theme`. The callback already assigns a fresh theme object to `terminal.options.theme`, which xterm applies in place. The settings value is never a dependency of the effect that creates and connects the terminal, so a theme change cannot recreate, reconnect, resubscribe, clear, or scroll it. Hidden terminals are still mounted, so the observer fires for them too and they show the right palette when revealed; a new terminal reads the attribute the provider set before first render.

### 4. Palette coverage

Both palettes define background, foreground, cursor, cursor accent, selection background and foreground, the sixteen ANSI colors, the frame border, and the scrollbar slider. The installed xterm exposes `scrollbarSliderBackground`, `scrollbarSliderHoverBackground`, and `scrollbarSliderActiveBackground` as theme fields, so the scrollbar is themed through the same object rather than through a `.xterm-viewport` rule, which the workspace style guard forbids. The light default text is a dark gray on a near-white canvas; the ANSI set is a light-background palette with readable yellow and white.

### 5. Persistence and validation

Native: a `CliTerminalTheme` enum with `parse`/`as_str`, a new `DesktopSettingKey::CliTerminalTheme`, a mutation variant, a field with `Dark` default, and a DTO string. It is one more row in the existing `settings` table, so no migration. On load an unparsable stored value is skipped by the existing loop and the default stands; on save an invalid value is rejected by `parse`. Frontend: `cliTerminalThemes` registry, `isCliTerminalTheme`, `normalizeAppSettings` fallback to `dark`, and the existing `validateSettingValue` refuses anything that normalization would have changed.

### 6. Independence from the application theme

The two settings are separate keys with separate side effects in `applySettings`; neither writes the other. `data-theme` and `data-cli-terminal-theme` are independent attributes.

## Risks / Trade-offs

- A CLI that paints its own truecolor backgrounds will keep its own look on a light canvas. This is documented in the compatibility note and is the boundary, not a defect to patch with filters.
- The root attribute is global state. Acceptable: the setting is application-level by definition and the selector scoping is what prevents bleed.
