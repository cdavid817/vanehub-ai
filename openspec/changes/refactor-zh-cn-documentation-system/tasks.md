## 1. Establish the fact baseline

- [x] 1.1 Record the bundle's self-declared limitation (no repository access, no commands run) and treat every item as a lead
- [x] 1.2 Check each `high-priority-fixes.md` item against code, registry, main specs, config, and tests; record verdict and evidence in `design.md`
- [x] 1.3 Capture the pre-change baseline of `docs:check`, `docs:test`, `docs:build` so later failures are attributable — all three exit 0 on `origin/main` at `657ae37c`

## 2. Phase 1 — deterministic fixes

- [x] 2.1 Replace `npm run tauri -- dev` with `npm run tauri:dev` in `README.md`, `README.zh-CN.md`, `README.ja.md`. The `$env:PATH` prelude stays: the block is fenced `powershell`, which already declares the shell it assumes
- [x] 2.2 Remove the "21 native bounded contexts" claim from all three READMEs
- [x] 2.3 Correct "24 个上下文" / "24 contexts" to 27 in both `native-contexts.md` chapters, matching `src-tauri/src/contexts/`
- [x] 2.4 Correct the local-media roadmap sentence in all three READMEs
- [x] 2.5 Shrink both README guide tables to grouped entry points inside the locale-scoped block (zh-CN); drop the 25-name provider table, keeping the counts and the endpoint-protocol split a reader cannot derive from the catalogue
- [x] 2.6 Verify every entry of `user-guide-SUMMARY.phase1.md` resolves to an existing file, then apply the regrouping — 36 entries, 36 files, no gap either way
- [x] 2.7 Verify every entry of `developer-guide-SUMMARY.phase1.md` resolves to an existing file, then apply the regrouping — 40 entries, 40 files, no gap either way
- [x] 2.8 Run `docs:check` and `docs:test` after the regrouping, before any prose edits

## 3. Phase 2 — capability calibration

- [ ] 3.1 `core-concepts.md`: 5 external CLI Agents + 1 built-in OnePiece, with the in-process/out-of-process distinction stated once
- [ ] 3.2 `use-cases.md`: replace the time-only correlation instruction with `runId`/`traceId`/`spanId`, keeping time as the fallback for paths that carry no ids
- [ ] 3.3 `worktree.md`: state what a worktree isolates and what it does not; remove "no extra sandbox logic is needed"
- [ ] 3.4 `troubleshooting.md`: replace the "memory is a host-level shared pool, isolation only by turning it off" answer with the scope/audience model
- [ ] 3.5 `cross-session-memory.md`: `name` is display metadata; the filename is `{id}.md`
- [ ] 3.6 `retrieval.md`: keep the tool-schema fact, replace the stale reason ("the shared pool has no slices") with the eligibility filter's real order
- [ ] 3.7 `skill-management.md` (dev): replace "无状态,纯文本,无需权限系统" with the governed-runtime posture
- [ ] 3.8 `agent-skills-architecture.md`: description is the primary discovery signal a host may use, not the only possible one
- [ ] 3.9 `lsp-code-intelligence.md` (user): one settings entry point, one tool count, both matching the registry
- [ ] 3.10 `permissions.md`: name all three projecting CLIs and state that Antigravity CLI has no native projection while remaining under host policy
- [ ] 3.11 `remote-and-im.md`: state the text-DM-only inbound execution scope
- [ ] 3.12 `runtime-boundaries.md` (zh **and** en): replace ACP-stdio with JSON-RPC over stdio throughout; keep ACP only where a real Agent Client Protocol is meant
- [ ] 3.13 `function-calling-architecture.md`: Assistants API sunset in past tense with Responses as the migration target
- [ ] 3.14 `multi-agent-architecture.md`: add the source link for the 15x token figure
- [ ] 3.15 Search for surviving instances of every corrected wording across `docs/`, both languages, and the OpenSpec specs

## 4. Phase 3 — structure and guards

- [ ] 4.1 Rewrite `docs/agent-infrastructure/README.md` on the protocols / patterns / methods boundary, with sourcing and maintenance rules
- [ ] 4.2 `git mv` the three CLI references to `docs/reference/cli/`, then update every inbound link, the README table, and any script inventory
- [ ] 4.3 Extend `validate-docs.mjs`: a bounded-context chapter's prose total must equal the directory count, or be absent
- [ ] 4.4 Extend `validate-docs.mjs`: every `npm run <script>` in a README must exist in `package.json`
- [ ] 4.5 Add unit tests for both guards, covering the wrong-total, no-total, and unknown-script cases
- [ ] 4.6 Update `docs/developer-guide/zh-CN/src/index.md` and `docs/user-guide/zh-CN/src/index.md` where they restate a corrected fact

## 5. Verification

- [ ] 5.1 `npm run docs:check`
- [ ] 5.2 `npm run docs:test`
- [ ] 5.3 `npm run docs:build`
- [ ] 5.4 `npm run contracts:check`
- [ ] 5.5 `npm run architecture:check`
- [ ] 5.6 `npm run docs:unit:test` covers the two new guards
- [ ] 5.7 `openspec validate refactor-zh-cn-documentation-system --strict` and `openspec validate --specs --strict`
- [ ] 5.8 Confirm the working tree is clean and no generated artifact is left modified
- [ ] 5.9 Classify every failure as introduced-here, pre-existing on `main`, or environmental, with the evidence for the classification
