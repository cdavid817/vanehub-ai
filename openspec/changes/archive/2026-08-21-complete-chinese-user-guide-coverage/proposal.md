## Why

`rebuild-project-documentation-topology` collapsed the documentation topology and declared `docs/user-guide/zh-CN/` the authoritative complete set. It was complete as a *topology*: 22 chapters, every SUMMARY entry resolving to a file, every chapter carrying substantive prose (~130k characters, no outline-only sections).

It was not complete as *coverage*. A capability-by-capability audit against `openspec/specs/` found eight delivered, user-facing capabilities with **zero mentions anywhere in the 22 chapters**:

| Capability | Shipped in |
| --- | --- |
| `goal-management` | #138 |
| `unified-todo-board` | #125 |
| `slash-command-runtime` | #130 |
| `agent-code-review` | #146 |
| `agent-evaluation` | #155 |
| `signed-desktop-auto-update` | #148 |
| `agent-notebook-editing` | #140 |
| `skill-evolution-evidence` | #126 |

Four more carried only one or two incidental mentions with no dedicated treatment: `agent-cross-session-memory` (#141), `agent-context-engine` / `agent-context-compaction` (#145, #128), file references (#133), and `session-recovery` (#117).

The cause is mechanical: the guide was authored before those capabilities merged, and nothing in the spec or the validators ties guide coverage to delivered capabilities. Chapter count, link validity, and README parity are all enforced; *does a delivered feature appear anywhere* is not. So the guide passed every check while silently going stale — the same failure mode as the `TBD` `Purpose` placeholders that motivated the previous change.

This is a `Truthful feature-state labeling` problem in its most severe form: a delivered capability with no label at all reads as a capability that does not exist.

## What Changes

- Six new authoritative Chinese chapters: 目标管理, 任务看板, 斜杠命令, 代码评审, 记忆与上下文, 版本更新.
- Four capabilities folded into the chapters that already own their subject rather than given standalone chapters: Notebook editing into `native-agent.md` (it is an OnePiece tool capability bound by the workspace boundary and Plan mode), file references into `first-session.md`, session recovery into `troubleshooting.md`, Skill evolution evidence into `skill-management.md`.
- `agent-evaluation` is deliberately **out of scope**: the evaluation and benchmark platform addresses contributors assessing Agent quality, not users performing tasks. It belongs to `native-developer-documentation`, and placing it in the user guide would misrepresent the audience boundary the guide spec draws.
- Each new Chinese chapter gains its English known-gap counterpart and both `SUMMARY.md` files gain the entry, so the declared transition in `user-guide-documentation` stays honest rather than silently omitting six chapters from the English navigation.
- `user-guide-documentation` gains a requirement that the authoritative guide must cover every delivered user-facing capability, with an explicit non-user-facing exclusion that has to be stated rather than assumed. This is what makes the audit repeatable instead of a one-time cleanup.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-guide-documentation`: gains a delivered-capability coverage requirement, so a capability that ships without guide coverage is a spec violation rather than an invisible gap; the existing equivalence, transition, and labeling requirements are unchanged.

## Impact

**Runtime scope: neither.** Documentation and spec only. No application code, no Tauri command, no frontend service, no runtime adapter, no SQLite migration.

Affected surfaces:

- `docs/user-guide/zh-CN/src/` — six new chapters, four existing chapters extended, `SUMMARY.md` updated.
- `docs/user-guide/en/src/` — six new known-gap chapters, `SUMMARY.md` updated.
- `openspec/specs/user-guide-documentation/spec.md` — one added requirement.

**The English guide's declared transition is extended, not closed.** Six chapters are added to the known-gap set; completing English content remains the follow-up this change does not attempt.
