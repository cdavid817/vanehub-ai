## 1. Relocate the group-chat implementation narrative

- [x] 1.1 Expand `docs/developer-guide/src/multi-agent-group-chat.md` to carry the migrated design: seat storage and its degradation behavior, handle derivation, the five handoff-parsing defenses with their source coordinates, the chain-limit constants, human-handoff intents and effects, model-family resolution, context delivery modes, the mirrored frontend/native implementation, and the code-location index. Keep the spec pointer as authoritative for requirements.
- [x] 1.2 Reduce `docs/user-guide/zh-CN/src/multi-agent-workflow.md` to user-observable behavior per design D1: no source coordinates, no Rust struct, no code-location index, no storage-design rationale.
- [x] 1.3 Apply the same reduction to `docs/user-guide/en/src/multi-agent-workflow.md`, keeping the two locales structurally equivalent.
- [x] 1.4 Confirm every behavior statement removed from the user guide is present in the developer guide, and that no statement was dropped from both — `SessionSeat`, `session_seat.rs`, `seat_turn.rs`, `MAX_CHAIN_DEPTH`, and the code index all appear in the developer guide and zero times in either user guide.

## 2. Reduce the tutorial to a walkthrough

- [x] 2.1 Remove the build-and-test apparatus from `docs/user-guide/zh-CN/src/multi-agent-testing-tutorial.md`, including the stale `openspec validate complete-multi-agent-session-presence` line. Step 1 now names the runtime to use instead of `npm run dev` / `npm run tauri:dev`.
- [x] 2.2 Apply the same reduction to the English tutorial — both went from 19 section headings to 15.
- [x] 2.3 Record the verification approach in the developer guide without naming a specific archived change.

## 3. Complete the English media set

- [x] 3.1 Make the Feishu selector in `tests/docs/documentation-screenshots.spec.ts` bilingual so an English run can reach the IM surface.
- [x] 3.2 Declare an `en` twin for every `zh-CN` scenario in `docs/user-guide/screenshots.json`, mirroring the Chinese path convention rather than the scenario id.
- [x] 3.3 Generate the captures with `npm run docs:screenshots:update` — 40 passed, 20 English and 20 Chinese files on disk.
- [x] 3.4 Reference the new English captures from 18 places across ten English chapters, matching the Chinese guide's placement, each with English alternative text.

## 4. Verification

- [x] 4.1 `npm run docs:check` passes.
- [x] 4.2 `npm run docs:test` passes.
- [x] 4.3 `npm run docs:build` passes — exit 0, with all 40 captures assembled under `user/assets/screenshots/`.
- [x] 4.4 `npm run docs:screenshots:check` passes — 40 passed, confirming the captures reproduce.
- [x] 4.5 `npm run lint:ci` passes.
- [x] 4.6 `openspec validate "align-user-guide-audience-and-media" --strict` passes; `openspec validate --specs --strict` reports 133 passed, 0 failed.
- [x] 4.7 Re-run the chapter-by-chapter heading comparison — 29/29 equivalent.
- [x] 4.8 Confirm no `.md` file under `docs/user-guide/` still cites a source path with a line range — zero files match.

## 5. Follow-on found while doing the above

- [x] 5.1 Two more stale English selectors in the capture harness had never been exercised because no English scenario existed to run them: `扩展能力` was paired with `Extensions` where the interface renders `Extension Capabilities`, and `IM 能力` with `Instant messaging` where it renders `IM Connectors`. Both fixed, and all 32 selector pairs in the harness were cross-checked against `src/i18n/locales/en.json` rather than fixed one failure at a time.
- [x] 5.2 `scripts/validate-docs.mjs` only resolved one hardcoded cross-book path, `../../developer/index.html`, so a deep link between books could not be link-checked. Generalized to `../../developer/<page>.html` and `../../user/<locale>/<page>.html` in both directions.
