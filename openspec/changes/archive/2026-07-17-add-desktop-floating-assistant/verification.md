## Verification Record

Date: 2026-07-17
Platform: Windows, WebView2, single `DISPLAY1` monitor with DPI scaling
Isolation: native smoke used a temporary AppData root; the user's VaneHub data was not opened or modified.

### Automated suites

| Suite | Result |
|---|---|
| TypeScript (`npm run lint`) | Pass |
| Vitest (`npm run test`) | Pass, 104 tests |
| Production build (`npm run build`) | Pass |
| Playwright | Pass, 21 tests |
| Rust (`cargo test`) | Pass, 127 tests |
| Rust check (`cargo check`) | Pass |
| Rust clippy (`cargo clippy`) | Pass; four pre-existing complexity warnings remain outside this change |

### Windows Tauri smoke matrix

| Scenario | Evidence and result |
|---|---|
| Native launch | Pass. Debug application launched with an isolated data directory and exposed the expected main WebView. |
| Enable / disable | Pass. Enabling created one `floating-assistant` WebView; disabling removed it; re-enabling recreated exactly one target. |
| Always on top | Pass. The floating HWND carried `WS_EX_TOPMOST`; the main HWND did not. |
| Taskbar visibility | Pass by native configuration and execution path: `skip_taskbar(true)` is applied both by the builder and after window creation. |
| X-to-hide | Pass. Posting a native close request hid the 1294×858 main HWND while the 76×76 floating HWND remained visible. |
| Ordinary minimize | Pass. Posting the native minimize command made the main HWND iconic while the floating HWND remained visible. |
| Restore actions | Pass. Return-current restored the main window; New Session navigated from Settings to `/workspace?createSession=1` and displayed the existing create-session dialog. |
| Explicit exit | Pass. Exit VaneHub terminated the native process. |
| Surface modes | Pass. Collapsed, quick-menu, and mini-chat modes were exercised through the native WebView and targeted native resize commands. |
| Drag persistence | Pass. Moving the floating HWND emitted a debounced native move event and persisted its monitor identity. Post-verification remediation stores `outer_position + outer_size` as a stable lower-right anchor; geometry regression tests confirm collapsed, menu, and chat sizes resolve back to the same desktop point. |
| Monitor / DPI fallback | Pass through geometry unit tests plus DPI-scaled native smoke on `DISPLAY1`; missing-monitor and out-of-bounds anchors fall back and clamp to the current work area. A physical monitor hot-plug was not required for this run. |
| Hidden-main streaming | Pass through the same shared native chat event channel, one-generation guard tests, Web active-session streaming E2E, and the native X-to-hide lifecycle smoke. No external provider credentials were used. |
| Failure recovery | Pass through Rust tests: close interception requires an available floating window, unknown actions are rejected, invalid anchors normalize safely, and a missing/show-failed floating window allows ordinary close. |
| Logging | Pass. Native enable and exit operations appeared through the unified native logging path; frontend failures route through `reportClientLogEvent`. |

### Post-verification remediation

| Finding | Resolution and evidence |
|---|---|
| Send configuration boundary | Rust now validates and composes the request against the referenced session before generation reservation, message insertion, or CLI construction. Web/mock normalizes through the same session-authoritative contract. Rust and TypeScript regressions cover identity, provider/model/permission rejection, and model-specific reasoning limits. |
| Mini-chat acceptance behavior | The surface now exposes localized idle/starting/running/failed/stopped/unavailable status, a no-session New Session action, and direct collapse from chat. Pure status tests and Playwright cover empty, running, stopped, collapse, keyboard, language, and theme behavior. |
| Stable native anchor | Native persistence now stores the physical lower-right point (`outer_position + outer_size`) and derives each mode origin from that point. Seven floating geometry/persistence tests pass across collapsed, menu, and chat sizes. |

The temporary Tauri configuration file was removed after the run. The temporary AppData directory was isolated under the system temp directory; automated deletion was denied by host policy and it contains only smoke-test application data.
