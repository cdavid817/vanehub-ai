## 1. Settings model and persistence

- [x] 1.1 Add `CliTerminalTheme` and `cliTerminalTheme` to `src/types/settings.ts`, defaults, `normalizeAppSettings`, and the reset key list.
- [x] 1.2 Add the native enum, key, mutation, field, getter, DTO field, and mapper output; extend the DTO contract and domain key tests.
- [x] 1.3 Add frontend tests for the default, the fallback on a missing or invalid stored value, rejection of an invalid write, and Web adapter round trip.

## 2. Theme and hot update

- [x] 2.1 Declare the light palette scoped to `.ucd-agent-terminal` under `data-cli-terminal-theme="light"`, including frame border and scrollbar slider colors.
- [x] 2.2 Let `createTerminalTheme()` read from a given element and include scrollbar slider fields; keep the no-argument call compatible.
- [x] 2.3 Apply `data-cli-terminal-theme` from `applySettings`; observe it in the Agent terminal and reassign the theme object in place.
- [x] 2.4 Add a component test proving a theme change reassigns the theme without creating a terminal, opening, stopping, or subscribing again, and that written output survives.

## 3. Settings UI and localization

- [x] 3.1 Add the "CLI 会话主题" row with the light/dark select and the compatibility note under the application theme in common preferences.
- [x] 3.2 Add the row's keys to all five locales.
- [x] 3.3 Extend the Basic Configuration page test for the new control and its save path.

## 3a. Follow-up hardening from the page review

- [x] 3a.1 Catch every select's save rejection on Basic Configuration and lock only the control being saved; remember the advanced disclosure's expanded state per viewer.
- [x] 3a.2 Show the CLI compatibility note only while light is selected, inside the row description.
- [x] 3a.3 Coalesce Agent terminal resize notifications per frame and send only grid changes; report input, resize, and stop failures through the tab's error state; drive the send control from terminal-id state.
- [x] 3a.4 Rebuild the terminal palette only on the CLI theme attribute, not on application theme changes.

## 4. End-to-end and documentation

- [x] 4.1 Add a Playwright spec that drives a real xterm in the Web runtime, switches the theme from settings, asserts the terminal canvas colors and that the same xterm element and its output survive, and saves light and dark screenshots.
- [x] 4.2 Note the setting in the architecture document's desktop settings summary.

## 5. Verification

- [x] 5.1 Run the validation commands from `AGENTS.md`.
- [x] 5.2 Run the Playwright spec and keep the screenshots.
- [ ] 5.3 Run the desktop settings-persistence layer, or report it as NOT RUN with the reason.
