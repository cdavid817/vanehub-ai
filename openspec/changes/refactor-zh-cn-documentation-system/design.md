## Context

The input is `vanehub-docs-refactor-bundle.zip` (7 Markdown artifacts + `audit-index.json`, 2026-09-02). Its index declares the constraint that governs how it must be used:

> "The repository could not be cloned in this execution environment." / "No local docs/test/build commands were executed."

So the bundle never observed the tree it audits and never ran the gates its recommendations would have to pass. Two consequences shaped this change:

1. **Every lead is checked before it is applied.** The fact hierarchy in the task brief is followed: code and registry first, then OpenSpec main specs, then generated references, then config and tests, then upstream specs, then the audit. Where the audit disagrees with code, code wins and the audit item is recorded as rejected.
2. **The bundle's own deliverables cannot be applied as patches.** `README.zh-CN.proposed.md` would fail `docs:readme:check` on its first run — it drops every `<!-- docs-section:… -->` marker and the `<!-- docs-locale-guides -->` block that `check-readme-parity.mjs` requires, and it changes shared relative links without touching the other two languages. It is used as a direction for section order and length, not as a replacement file.

### Constraints discovered in the repository, not in the bundle

- **README parity is mechanical.** `scripts/check-readme-parity.mjs` compares, across all three READMEs: `docs-section` marker sequence, fenced code blocks **verbatim**, repository-relative link targets **outside** the `docs-locale-guides` block, `docs-fact` markers, and the presence of exactly one locale-guides block. A zh-CN-only command fix therefore fails CI. Correcting `npm run tauri -- dev` is necessarily a three-file change.
- **The locale-guides block is the only place a translation may diverge structurally.** Both README guide tables live inside it, which is what makes Phase 1's table shrink possible in zh-CN alone.
- **Context tables are already enforced; prose is not.** `validate-docs.mjs` compares every backticked context name in `native-contexts.md` against `src-tauri/src/contexts/`, and its own unit test asserts that a context named *only in prose* is not counted. That is why 27 directories coexisted with a "24 contexts" sentence and a passing build.

## Goals / Non-Goals

**Goals.** Remove statements that are false against code; give each cross-cutting capability one definition; regroup both guide tables of contents without moving files; convert two hand-maintained fact classes into checked ones.

**Non-Goals.** Splitting `user-interface.md` / `tooling.md` / `remote-and-im.md` / `automation.md`, merging `goal-management.md` with `todo-board.md`, splitting `execution-observability.md` and `persistence-and-logging.md`, and adding per-file YAML frontmatter. These are the audit's Phase 3 body. They are deferred, with reasons in "Deferred work" below — not because they are wrong, but because each rewrites documents whose *content* is currently accurate, and bundling accurate-content rewrites with false-statement repairs makes the latter unreviewable.

## Decisions

### Decision 1: fix commands in all three READMEs, restructure only zh-CN

Parity forces the command fix to be trilingual. The task brief says not to overwrite the English or Japanese README, which is about wholesale replacement, not about targeted repairs. The rule applied here:

- A defect that parity compares (commands, shared links, section markers, facts) → fixed in all three.
- A prose defect that exists verbatim in all three (the bounded-context total, the local-media roadmap wording) → fixed in all three, because repairing one language and leaving a known falsehood in the other two manufactures new drift.
- Structural shrinking of the guide tables → zh-CN only, inside the exempt block.

### Decision 2: make the two drifted fact classes checked rather than re-hardcoding them

`native-contexts.md`'s total and README `npm run` commands both drifted while every gate stayed green. Re-writing "27" would restore the same failure mode on the next context. Instead `validate-docs.mjs` gains two guards:

- The bounded-context chapters must state a total that equals the directory count, or state no total at all. Both phrasings pass; a *wrong* total fails.
- Every `npm run <script>` in a README must exist in `package.json` `scripts`.

This is the minimum code change that satisfies "fix the documentation checker to eliminate deterministic drift" without expanding into business code.

### Decision 3: resolve the `skill_evolution_evidence` encryption conflict on the specification side

`openspec/project.md` described that context's ownership as "encrypted evidence storage". `skill-evolution-evidence.md:90` already stated that no encryption layer exists. The conflict was first recorded here for a decision, and that decision has now been taken: **the specification wording changes, the implementation does not.**

| | |
| --- | --- |
| **Implementation** | `storage_values.rs` converts between enums and strings; the schema and repository perform no encryption. Evidence rows rest on whatever the OS and disk provide, after sanitization at write time. |
| **Former specification** | `openspec/project.md` named the ownership "encrypted evidence storage". |
| **Resolution** | The ownership description now states sanitization-before-write plus OS and disk protection, and says explicitly that there is no application-level encryption layer. Four documents carried the claim — `openspec/project.md`, both `native-contexts.md` ownership tables, and the two evidence chapters — and all four are corrected. |
| **Why this side** | Adding encryption at rest is a security feature, not a wording change: it needs key management, migration of existing rows, and erasure verification, and it would land in a documentation change with none of that reviewed. Asserting a protection that does not exist is the worse of the two states to leave standing, and it is the one a wording change can actually fix. |
| **Business code change required** | No. Raising evidence to application-level encryption remains open as its own piece of work, and the chapters now say what that would take. |

### Decision 5: state the memory-audience boundary instead of repeating the product promise

Correcting `troubleshooting.md`'s "you cannot isolate memory per Agent" turned up something the audit did not look for. Two capabilities govern two different paths, and only one of them enforces audience:

| Path | Enforcement | Authority |
| --- | --- | --- |
| Injection into a prompt (OnePiece context assembly, CLI index) | `personalization::domain::memory::eligibility()` excludes by status, read policy, scope, then audience — before budgeting or relevance selection | `agent-cross-session-memory` |
| The `recall` tool | None. `retrieval` contains no reference to audience or `eligibility`; `vector_candidates` and `keyword_candidates` span the whole shared pool, and `ports.rs:66` documents that as intentional | `retrieval-vector-search` |

The two main specs do not literally contradict each other — `agent-cross-session-memory`'s exclusion scenarios are all about injection (the CLI index, the Context Engine budget, the Web mock), and `retrieval-vector-search` explicitly requires that "recall SHALL NOT return a strict subset of what memory injection already placed in the system prompt". The design is deliberate: a recall tool that re-filtered by audience could only return what injection already supplied.

What is wrong is the documentation. `faq.md` answered "can one Agent have separate memory?" with an unqualified yes, and `personalization.md` presented restricted audience as the solution to per-Agent isolation. Neither said that the restriction governs injection only, so a reader would reasonably conclude that an excluded Agent cannot reach the memory. It can, through `recall`.

| | |
| --- | --- |
| **Current implementation** | Audience and scope filter injection. The recall tool searches the entire host-level pool. |
| **Current specification** | Both behaviours are specified, in two capabilities, with no requirement reconciling them at the boundary. |
| **Impact** | A user who restricts a memory's audience for confidentiality is not getting confidentiality. The exposure is local (same host, same user account) but it is not what the guide promised. |
| **Proposed resolution** | Either (a) accept the boundary and keep it stated wherever audience is offered — which is what this change does — or (b) add a requirement that recall respects audience for records whose audience is explicitly restricted, leaving all-Agent records unfiltered. Option (b) preserves the "not a strict subset" property while closing the gap. |
| **Business code change required** | No under (a); yes under (b). This change makes neither choice, and documents the boundary in `troubleshooting.md`, `faq.md`, `personalization.md`, and `retrieval.md` so it is not discovered by accident. |

### Decision 4: keep the settings entry point that the registry proves

`lsp-code-intelligence.md` names two entry points. `src/settings/settings-pages.ts:96` registers `id: "code-intelligence"` with `labelKey: "settings.pages.codeIntelligence"`, which `src/i18n/locales/zh-CN.json:46` resolves to **代码智能**. "设置 → Agent 配置 → 语言服务器智能" matches no registered page. The registry wins; the second reference is corrected, not the first.

## Revision decision table

Verdicts: **采纳** adopt as proposed · **调整** adopt with correction · **拒绝** reject (claim does not hold) · **待确认** unresolved.

### From `high-priority-fixes.md`

| # | Audit proposal | Files | What the document actually said | Source of truth | Verdict | Result |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `npm run tauri -- dev` → `npm run tauri:dev` | 3 READMEs | `npm run tauri -- dev` (line 201 in each) | `package.json` registers `tauri:dev`; no `tauri` script | **调整** | Fixed in all three, not just zh-CN — parity compares code blocks verbatim. Audit named only the Chinese README. |
| 2 | Stop hardcoding bounded-context totals | 3 READMEs, both `native-contexts.md` | READMEs "21"; both guides "24 个上下文" | `src-tauri/src/contexts/` = **27** dirs | **调整** | Audit reported 21-vs-24 as README-stale-against-guide. Both are stale against the tree. READMEs drop the number; guides state the real total and it becomes checked. |
| 3 | "内置 6 个 Agent" → 5 CLI + 1 OnePiece | `core-concepts.md:25` | "VaneHub AI 内置 6 个" | 5 external CLIs are spawned processes; OnePiece is in-process | **采纳** | Reworded; the capability-status list the audit proposed is folded into the existing per-Agent table rather than added as a second one. |
| 4 | Unify memory scope/audience/candidate/immutable-id | `troubleshooting.md`, `retrieval.md`, `cross-session-memory.md`; audit also named `faq.md`, `personalization.md`, `memory-and-context.md` | `troubleshooting.md:130` "只能整体关闭记忆"; `retrieval.md:11` "共享池没有可供模型指名的切片"; `cross-session-memory.md:92` "`name`…与文件名 `{name}.md` 对应" | `memory.rs`: `MemoryScope::{Global,Workspace}`, `MemoryAudience::admits()`, `MemoryStatus::{Candidate,Active,Archived}`, `eligibility()`; `memory.rs:285` `format!("{}.md", self.id)` | **调整** | 3 of the 6 named files were already correct — `faq.md:17` and `personalization.md:88` describe the new model. Fixed only the 3 that were stale. |
| 5 | Replace "logs correlate by time only" | `use-cases.md:141` | "日志中刻意不包含链路标识符…用**时间**把两边对上，而不是用 id" | `runtime_support.rs:378-384` inserts `runId`/`traceId`/`spanId` into log context | **调整** | Confirmed as a defect, but in a different file. The audit named `observability.md`, `execution-observability.md`, and `persistence-and-logging.md`; all three were already correct and cite the adapter. |
| 6 | One LSP capability table from the registry | `lsp-code-intelligence.md` (user) | Line 3 "设置 → 代码智能" vs line 181 "设置 → Agent 配置 → 语言服务器智能"; line 63 "九个只读工具" vs line 189 "四个 LSP 工具" | `settings-pages.ts:96` + `zh-CN.json:46` = 代码智能; the chapter's own tool table lists 9 | **调整** | Internal contradictions fixed against the registry. Generating the table from code is deferred — no single registry exposes language × server × tool × integrity today; see Deferred work. |
| 7 | Worktree is not a security sandbox | `worktree.md:73` | "不需要额外写沙箱逻辑…操作系统层面的目录隔离本身就是边界" | A worktree is a checkout; it constrains no file access, network, or credentials | **采纳** | Replaced with what a worktree does constrain, and what still has to be enforced separately. |
| 8 | IM first release executes text DMs only | `remote-and-im.md` (user) | No statement either way | `protocol.rs:38,66,102` gate on `p2p`/`private`/`single`; `runtime_manager.rs:716` handles `group-message` separately | **调整** | The developer guide (`im-connectors.md:11,15`) was already correct and matches code. The user guide omitted the limit rather than overstating it; the scope statement is added there. |
| 9 | Don't claim uniform CLI permission projection | `permissions.md:77` | "其余三个 CLI 各有各的表达方式——OpenCode 用环境变量，Codex CLI 用命令行选项" | `permission-model.md:61` — projection covers `gemini-cli`, `codex-cli`, `opencode`; Claude Code uses a hook bridge | **调整** | The quoted claim ("all CLIs support the same Trust/Yolo projection") does not exist. The real defect: "其余三个" names two of the three and never accounts for Antigravity CLI. Fixed. |
| 10 | Drop Skill absolutes | `skill-management.md:16` (dev), `agent-skills-architecture.md:112` | "无状态,纯文本,无需权限系统"; "description 是唯一的触发信号" | `skill-tool-runtime-security.md` documents a Wasmtime sandbox with fuel, epoch interruption, store limits, trust, integrity, kill switches | **调整** | Both absolutes corrected. The audit's other claims — "unsourced ecosystem numbers", "million skills", "fixed benchmark figures" — do not appear; line 59's token figures carry their measurement method. Rejected. |
| 11 | Assistants API is past tense | `function-calling-architecture.md:5,202` | "已废弃并定于 2026-08-26 关停" | Sunset date is in the past relative to 2026-09-02 | **采纳** | Rewritten as completed, with Responses API as the migration target. |
| 12 | "ACP-stdio" → "JSON-RPC over stdio" | `runtime-boundaries.md` (zh **and** en) | Section heading, comparison table, and 6 further uses treat ACP as the generic name for LSP/MCP stdio framing; line 108 even glosses it as "Agent 通信协议(ACP)" | LSP and MCP each define their own JSON-RPC-over-stdio binding; neither is ACP | **调整** | Audit named only the Chinese file. The English guide carries the same error and is fixed too. |
| 13 | Calibrate local-media status | 3 READMEs | Roadmap: "扩展的本地 OCR/语音能力" — reads as not yet delivered | `contexts/local_media/` ships OCR/STT/TTS with a real engine bridge; `local-media.md:3,16,58` documents no cloud fallback and canary verification | **调整** | Fixed in the roadmap line of all three READMEs. The user chapter the audit also flagged was already accurate and is unchanged. |
| 14 | Escalate the evidence-encryption contradiction | `skill-evolution-evidence.md:90` | Already states the conflict inline | `storage_values.rs` has no encryption layer; `openspec/project.md` says "encrypted evidence storage" | **待确认** | Recorded in Decision 3 as an implementation-vs-spec conflict with a proposed resolution. Not resolved here; resolving it needs business code or a main-spec amendment. |
| 15 | External-citation rules per document | `agent-infrastructure/README.md` | No stated sourcing rules | — | **采纳** | Added to the directory README as maintenance rules. |

### From `vanehub-docs-audit.md` and the candidate files

| Audit proposal | Verdict | Result |
| --- | --- | --- |
| `README.zh-CN.proposed.md` as a replacement file | **拒绝** as a file, **采纳** as direction | Applying it fails `docs:readme:check`: no `docs-section` markers, no `docs-locale-guides` block, shared links changed in one language only. Its section order and its "route, don't reproduce" principle are applied to the existing scaffold. |
| Drop the ~35-row user-guide and ~25-row developer-guide README tables | **采纳** | Both replaced with grouped entry points, inside the locale-scoped block. |
| Drop the provider catalogue from the README | **调整** | The 25-provider list is dropped; the endpoint-protocol sentence stays, because "which Agent can use which provider" is the one thing a reader cannot derive from the linked catalogue. |
| `user-guide-SUMMARY.phase1.md` | **调整** | Applied after verifying all 36 entries resolve to existing files. Two of its group names are changed: `local-media.md` moves out of "工作区与会话" (it is a tool, not a workspace concern) and `code-review.md` out of it as well (it belongs with Agent collaboration). |
| `developer-guide-SUMMARY.phase1.md` | **调整** | Applied after verifying all 39 entries. `session-workspace-console.md` stays under observability rather than moving to platform capabilities — it is an evidence surface, and the audit's own developer matrix says so two rows later. |
| `agent-infrastructure-README.proposed.md` | **采纳** | Adopted as the boundary restatement, minus its YAML frontmatter block (see Deferred work). |
| Move the three CLI references to `docs/reference/cli/` | **采纳** | Done with `git mv`; all inbound links, the README table, and `validate-docs.mjs` inventories updated. |
| Re-group agent-infrastructure into `protocols/` `patterns/` `methods/` | **拒绝 (this change)** | The directory README now states the grouping, which is the part that helps a reader. Moving 10 files changes 40+ inbound links across both guides and both languages for no reader-visible gain beyond what the grouped README already provides. Deferred. |
| Per-file YAML frontmatter (`audience`/`status`/`source_of_truth`/…) | **拒绝** | mdBook renders unrecognised leading YAML as body text; `docs:test` walks every chapter and `docs:build` publishes it. The audit's own instruction says not to break the build if incompatible. Status is expressed in prose instead. |
| `docs/facts.json` generator for agents, providers, LSP, permissions, UI surfaces, limits, packaging | **拒绝 (this change)** | Two of the nine fact classes actually drifted; both are now checked at their existing source. A new generated-facts file for the other seven is a speculative structure with no observed failure behind it. |
| "15x token" needs a source | **调整** | The figure is already attributed to Anthropic's published research; the citation link is added rather than the figure removed. |
| Lint for "共 N 个/支持 N 种" phrases with an allowlist | **拒绝** | A phrase-shaped lint over Chinese prose flags every legitimate enumeration. The two totals that drifted are now checked against their real source, which is stronger and produces no allowlist. |
| `last_verified` staleness warnings at 180 days | **拒绝 (this change)** | Depends on the rejected frontmatter. |

### Decision 6: `docs/reference/cli/` overlaps a build-time namespace — accepted, with the trap written down

The brief and the audit both name `docs/reference/cli/` as the destination for the three CLI documents, and that is where they now live. One thing to know before adding more there:

`docs/reference/` did not exist in source. It is assembled by `build-docs.mjs`, which copies exactly two files into it — `docs/release-signing.md` and `src-tauri/ARCHITECTURE.md` — and `validate-docs.mjs` special-cases `../reference/release-signing.md` and `../reference/native-architecture.md` in developer-guide files so those authored links resolve. So `../reference/X` written in the developer guide and `docs/reference/X` on disk are now two different namespaces that happen to share a name.

This is survivable because the special cases are exact-match on two filenames and only apply to developer-guide files, so the new directory resolves normally from everywhere else. It becomes a problem if someone adds a third build-time `reference/` entry whose name collides with a real file under `docs/reference/`. Neither location is published to the assembled site today — `docs/agent-infrastructure/` was not either — so the move changed the GitHub browsing path and nothing else.

## Deferred work

Recorded so the next change does not rediscover it:

1. **Generated LSP capability matrix.** Blocked: language, server binary, install strategy, integrity state, and negotiated tool set live in different places, and the negotiated set is a runtime property of each server handshake, not a static registry value. A generator would have to declare a static approximation of something the chapter correctly describes as negotiated.
2. **Splitting `user-interface.md`, `tooling.md`, `remote-and-im.md`, `automation.md`; merging `goal-management.md` + `todo-board.md`; splitting `execution-observability.md` and `persistence-and-logging.md`.** These are size and organisation problems, not correctness problems.
3. **`docs/agent-infrastructure/` subdirectory move.** See the table above.
4. **`skill_evolution_evidence` encryption.** See Decision 3.

## Risks / Trade-offs

- **Regrouping a SUMMARY changes every chapter's `prev`/`next`.** Mitigated by `docs:test`, which walks the whole book, and by not renaming any file — so no external link breaks.
- **The new prose-total guard could false-positive on an unrelated number.** Mitigated by anchoring the pattern to the bounded-context sentence in the two known chapters and unit-testing both the wrong-total and no-total cases.
- **Fixing English and Japanese prose exceeds a "Chinese documentation" brief.** Accepted: parity makes part of it mandatory, and the remainder is a known falsehood in a shipped document.
