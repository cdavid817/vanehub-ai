## 1. Audit

- [x] 1.1 Cross-check every capability under `openspec/specs/` against `docs/user-guide/zh-CN/src/` and record which delivered user-facing capabilities have zero or incidental coverage.
- [x] 1.2 Classify each gap as standalone chapter or fold-in per design D1, and record the `agent-evaluation` exclusion per design D2.

## 2. New authoritative Chinese chapters

- [x] 2.1 `goal-management.md` — 目标管理: derived 待验收 state, the plan-versus-loop terminal-state divergence, Session/Run links excluded from derivation, manual-only acceptance, unresolvable-child degradation.
- [x] 2.2 `todo-board.md` — 任务看板: board stage decoupled from projected source status, idempotent reconciliation, child-Session suppression, archived items not re-created, archive/delete never touching sources.
- [x] 2.3 `slash-commands.md` — 斜杠命令: the twenty commands, the recognition rules including the path counter-example, the `//` escape and why it must exist, session-scoped availability, output rendered outside the message list.
- [x] 2.4 `code-review.md` — 代码评审: fingerprint-anchored comments, accept-records-no-write, the guarded fail-closed revert, structured feedback with stale acknowledgement, metadata-only logging.
- [x] 2.5 `memory-and-context.md` — 记忆与上下文: the shared host-level pool, addressable memory files, the two toggles and what each does not cover, age and staleness annotation, Token-aware compaction with character fallback, context health disclaimers.
- [x] 2.6 `app-updates.md` — 版本更新: channel and downgrade policy, signature verification, endpoint/key immutability, failure recovery preserving the running install, no automatic restart.

## 3. Fold-ins to existing chapters

- [x] 3.1 Notebook cell editing into `native-agent.md`: cells rather than notebook JSON, output bytes never in the read result, output/execution-count clearing on source change, refusal on non-notebooks, Plan-mode read-only.
- [x] 3.2 File references into `first-session.md`: range and whole-file reference, the five-reference limit verified against both frontend and native constants, referenced content counting toward request Tokens, the three preview-unavailable states.
- [x] 3.3 Session recovery into `troubleshooting.md`: the three recovery outcomes, business-evidence-only reconciliation, `action_required` acknowledgement semantics, no automatic replay, idempotence.
- [x] 3.4 Skill evolution evidence into `skill-management.md`: the evidence-only boundary, six deterministic extractors, cancellation classified neutral, twelve-class sanitization before fingerprinting, collection failure never failing Agent work, purge scope.

## 4. English transition parity

- [x] 4.1 Create the six English known-gap chapters mirroring the existing stub format and linking to their Chinese counterparts.
- [x] 4.2 Add all six entries to `docs/user-guide/en/src/SUMMARY.md` and `docs/user-guide/zh-CN/src/SUMMARY.md` in matching navigation order.

## 5. Spec sync

- [x] 5.1 Sync this change's `user-guide-documentation` delta into `openspec/specs/user-guide-documentation/spec.md`.
- [x] 5.2 Record the `agent-evaluation` exclusion where the new requirement's second scenario expects to find it (design D2).

## 6. Verification

- [x] 6.1 `npm run docs:check` passes.
- [x] 6.2 `npm run docs:test` passes (mdBook test on all surviving books).
- [x] 6.3 `npm run docs:build` produces the assembled site including the six new chapters.
- [x] 6.4 `openspec validate "complete-chinese-user-guide-coverage" --strict` passes.
- [x] 6.5 `openspec validate --specs --strict` passes after the sync in 5.1 — 133 passed, 0 failed.
- [x] 6.6 Re-run the audit from 1.1 and confirm every in-scope gap is closed.
