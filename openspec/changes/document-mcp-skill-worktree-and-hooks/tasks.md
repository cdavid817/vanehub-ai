## 1. MCP

- [x] 1.1 Write `mcp.md` in both locales: what MCP is, what it can and cannot do, the configuration model, the three transports, the naming rules, registration and testing, import/export, the relay's scope, per-call approval, and the resource limits.
- [x] 1.2 Record the facts that had no coverage: plaintext storage and export of environment variables and headers; every tool call requiring explicit approval with no automatic path; server visibility re-validated immediately before connecting; an unrecognized transport never being treated as `stdio`; status being read from cache rather than a live connection.

## 2. Prompt Hook

- [x] 2.1 Write `prompt-hooks.md` in both locales: what a Hook is, what it can and cannot do, built-in versus user Hooks, the seven categories and two stages, the variable allowlist, and the draft/publish/rollback lifecycle.
- [x] 2.2 Record the load-bearing constraints: unknown variables rejected at publication rather than at assembly; substitutions preserved as inert text and never executed; `current_time` taking one snapshot per assembly; rollback appending a new version instead of rewriting history, and leaving an unpublished draft untouched.

## 3. Git worktree

- [x] 3.1 Write `worktree.md` in both locales: what a worktree is, how it differs from a clone, what it solves here, when it is available, how path and branch are derived, and the three cases rejected before any Git command runs.
- [x] 3.2 Cover the Loop worktree as a separate arrangement, including that it is never cleaned up automatically after any terminal state.

## 4. Skill and Loop Engineering

- [x] 4.1 Add an opening to `skill-management.md` in both locales: what a Skill is, how it differs from custom instructions and from MCP, what it can and cannot do, and enabled-versus-assigned as an explicit premise.
- [x] 4.2 Add definition, composition, and boundaries to `loop-engineering.md` in both locales, including the Worker/Verifier split, the structured verification command record, and the save-time rejections.

## 5. Multi-Agent topologies

- [x] 5.1 Replace the three-row pattern comparison in `multi-agent-workflow.md` with the seven-topology classification, marking VaneHub's position in every row including the unimplemented ones.
- [x] 5.2 State the two mappings that are not a plain tick: group chat as a shared-pool/peer-handoff hybrid with no scheduler, and Loop as a runtime-driven pipeline — the latter being why the two mechanisms share no orchestration logic.

## 6. Navigation and reuse

- [x] 6.1 Reduce `tooling.md`'s MCP and Prompt Hook sections to pointers in both locales.
- [x] 6.2 Move the two screenshots those sections carried into the new chapters; confirm zero orphaned captures in either locale.
- [x] 6.3 Add the three chapters to both `SUMMARY.md` files in matching order, and to both `index.md` tables.

## 7. Correct label mismatches found while writing

- [x] 7.1 `loop-engineering.md`: phases `行动` → **执行** and `判定` → **决策**; control `取消` → **停止**.
- [x] 7.2 `tooling.md`: the MCP state set listed `已断开`, which does not exist; the real set is 测试通过 / 测试失败 / 未测试 / 已禁用.
- [x] 7.3 `loop-engineering.md`: creation is a four-step wizard, not a flat five-step list; a run has seven states; acceptance offers three choices including the previously undocumented **根据反馈继续**.

## 8. Verification

- [x] 8.1 `npm run docs:check` passes, including anchors.
- [x] 8.2 `npm run docs:test` passes.
- [x] 8.3 Both `SUMMARY.md` files match in chapter set and order — 32 chapters.
- [x] 8.4 Both `index.md` tables cover every chapter in their `SUMMARY.md` — 31 each, excluding the index itself.
- [x] 8.5 Chapter-by-chapter heading comparison across all 32 chapters — zero divergence.
- [x] 8.6 Zero orphaned screenshots in either locale.
- [x] 8.7 `npm run docs:build` passes — exit 0, all three new chapters rendered in both books, zero broken image references in the built site.
- [x] 8.8 `openspec validate "document-mcp-skill-worktree-and-hooks" --strict` and `openspec validate --specs --strict` pass — 135 passed, 0 failed.
