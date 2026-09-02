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

- [x] 3.1 `core-concepts.md`: 5 external CLI Agents + 1 built-in OnePiece, with the in-process/out-of-process distinction stated once
- [x] 3.2 `use-cases.md`: replace the time-only correlation instruction with `runId`/`traceId`/`spanId`, keeping time as the fallback for paths that carry no ids
- [x] 3.3 `worktree.md`: state what a worktree isolates and what it does not; remove "no extra sandbox logic is needed"
- [x] 3.4 `troubleshooting.md`: replace the "memory is a host-level shared pool, isolation only by turning it off" answer with the scope/audience model
- [x] 3.5 `cross-session-memory.md`: `name` is display metadata; the filename is `{id}.md`
- [x] 3.6 `retrieval.md`: keep the tool-schema fact, replace the stale reason ("the shared pool has no slices") with the eligibility filter's real order
- [x] 3.7 `skill-management.md` (dev): replace "无状态,纯文本,无需权限系统" with the governed-runtime posture, linking the two chapters that document it
- [x] 3.8 `agent-skills-architecture.md`: description is the primary discovery signal a host may use, not the only possible one
- [x] 3.9 `lsp-code-intelligence.md` (user): one settings entry point, one tool count, both matching the registry. `语言服务器智能` is real (`lspSettings.title`) — it is the section inside the 代码智能 page, so only the page group was wrong
- [x] 3.10 `permissions.md`: name all three projecting CLIs and state that Antigravity CLI has no native projection while remaining under host policy
- [x] 3.11 `remote-and-im.md`: state the text-DM-only inbound execution scope
- [x] 3.12 `runtime-boundaries.md` (zh **and** en): replace ACP-stdio with JSON-RPC over stdio throughout; keep ACP only in the note that says what it actually is
- [x] 3.13 `function-calling-architecture.md`: Assistants API sunset in past tense with Responses as the migration target, verified against OpenAI's own deprecation announcement
- [x] 3.14 `multi-agent-architecture.md`: add the source link for the 15x token figure
- [x] 3.15 Search for surviving instances of every corrected wording across `docs/`, both languages, and the OpenSpec specs

### Found while doing 3.6 and 3.12 — not in the audit

- [x] 3.16 Record the memory-audience/recall boundary as an unresolved cross-capability conflict (`design.md`, Decision 5) and state it in all four documents that offer audience as isolation
- [x] 3.17 Correct the claim that LSP and MCP share one `Content-Length` framing. `read_bounded_frame` delimits MCP frames on `\n`; only LSP uses `Content-Length`
- [x] 3.18 Apply every Phase 2 correction to the English user guide and developer guide, which carried each defect verbatim — a false security boundary and a wrong diagnostic technique cannot be left standing in one language because the brief named another

## 4. Phase 3 — structure and guards

- [x] 4.1 Rewrite `docs/agent-infrastructure/README.md` on the protocols / patterns / methods boundary, with sourcing and maintenance rules
- [x] 4.2 `git mv` the three CLI references to `docs/reference/cli/`, then update every inbound link, the README table, and any script inventory — also the generator's `TARGET` and the `.gitattributes` eol rule; the link checker caught two `tooling.md` links the manual sweep had missed
- [x] 4.3 Extend `validate-docs.mjs`: a bounded-context chapter's prose total must equal the directory count, or be absent
- [x] 4.4 Extend `validate-docs.mjs`: every `npm run <script>` in a README must exist in `package.json`
- [x] 4.5 Add unit tests for both guards, covering the wrong-total, no-total, and unknown-script cases
- [x] 4.6 Update the developer-guide index chapters in both languages, which carried a third copy of the "21 contexts" claim. `user-guide/zh-CN/src/index.md`'s "九个标签页" was checked against `sessionTabDefinitions` and is correct — left alone
- [x] 4.7 Prove both guards fail on the real defects, not only on synthetic input: reintroduce "24 个上下文" and `npm run tauri -- dev`, confirm `validate-docs.mjs` reports both, then revert

## 5. Verification

- [x] 5.1 `npm run docs:check` — PASSED (47 unit tests, README parity, link/media/boundary inventories)
- [x] 5.2 `npm run docs:test` — PASSED (all four mdBooks)
- [x] 5.3 `npm run docs:build` — PASSED, exit 0, including the assembled-site validation
- [x] 5.4 `npm run contracts:check` — PASSED (catalog and matrix up to date at the new path, 16 tests)
- [x] 5.5 `npm run architecture:check` — PASSED, exit 0 (`lint:ci`, `tsc --noEmit`, architecture fitness tests, four Rust suites)
- [x] 5.6 `npm run docs:unit:test` covers the two new guards — 40 tests before, 47 after
- [x] 5.7 `openspec validate refactor-zh-cn-documentation-system --strict` — valid; `openspec validate --specs --strict` — 147 passed, 0 failed
- [x] 5.8 Working tree clean. `src-tauri/gen/schemas/*.json` showed as dirty after the cargo runs but carried no content change, and git's normalization kept them out of the commit
- [x] 5.9 No failure to classify: every gate passed. The two failures seen during the work were deliberate — the injected defects in 4.7 — and one real miss the link checker caught, the two `tooling.md` links to the moved matrix

## 6. Phase 3 — structure

Deferred at first, then requested. Both guides are split together: `user-guide-documentation` requires two complete, equivalent guides, so splitting one language alone would make them structurally diverge.

- [x] 6.1 Resolve the `skill_evolution_evidence` encryption conflict on the specification side (`design.md`, Decision 3). Four documents carried the claim, not the one the earlier note named
- [x] 6.2 Regroup `docs/agent-infrastructure/` into `protocols/`, `patterns/`, `methods/`, dropping the `-architecture` suffix the directory now carries itself
- [x] 6.3 Split `user-interface.md` into interface / session workspace / settings
- [x] 6.4 Split `tooling.md` into `agent-configuration.md` and `extensions.md`
- [x] 6.5 Split `remote-and-im.md` into `remote-workspaces.md` and `im-connectors.md`
- [x] 6.6 Split `automation.md` into `scheduled-tasks.md` and `usage-statistics.md`, distributing the shared "Notes and limits" bullets to whichever half each one constrains
- [x] 6.7 Merge `goal-management.md` and `todo-board.md` into `goals-and-work-board.md`
- [x] 6.8 Move `multi-agent-testing-tutorial.md` to the developer guide as `multi-agent-acceptance.md`
- [x] 6.9 Split `execution-observability.md` into observability and `evaluation-runtime.md`
- [x] 6.10 Split `persistence-and-logging.md` into `persistence-ownership.md` and `unified-logging.md`, splitting the constants section with it
- [x] 6.11 Split `testing-and-release.md` into `testing.md` and `release.md`, and drop the verbatim copy of AGENTS.md's verification list in favour of a reference
- [x] 6.12 Rebuild all four `SUMMARY.md` files and both `index.md` navigation tables
- [x] 6.13 Give the English and Japanese READMEs the grouped entry points the Chinese one already had, rather than rewiring the sixteen anchored links each had into the split chapters
- [x] 6.14 Drop the duplicated H1/H2 pair a whole-section-to-file split leaves behind, promoting the subheadings it held

### Found while doing 6.9

- [x] 6.15 `execution-observability.md` carried the same time-only correlation defect as `use-cases.md`, phrased in reverse ("traces deliberately carry no log identifier") — which is why the Phase 2 sweep missed it. Both languages corrected

## 7. Still open

- [ ] 7.1 Decide whether `recall` should respect an explicitly restricted memory audience (`design.md`, Decision 5). Until then the documentation states the boundary rather than the promise
- [ ] 7.2 Raise skill-evolution evidence to application-level encryption, if that is wanted: key management, migration of existing rows, erasure verification. The chapters now say what it would take
- [ ] 7.3 Generate the LSP capability matrix from a registry rather than maintaining it in the chapter (`design.md`, Deferred work 1)
