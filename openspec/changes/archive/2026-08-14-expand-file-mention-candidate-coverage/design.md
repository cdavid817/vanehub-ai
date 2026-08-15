## Context

See `proposal.md` - Why for the motivation. The constraints that shape the approach:

- `list_session_documents` walks the session root through `collect_documents`, bounded by a depth limit, a 300-entry cap, and an extension match that admits only `md`/`markdown`/`txt`. Dot-prefixed entries are already skipped; vendored trees such as dependency installs are not.
- That command serves the Documents tab, whose requirement in `session-workspace-tabs` explicitly scopes it to Markdown and text documents. It cannot be widened without changing a requirement this change does not intend to touch.
- The chat composer receives the same listing as a fully preloaded array and filters it client-side with a substring test.
- `ChatInputBox.tsx` is 188 lines against the repository's 300-line ESLint cap, which admits no new exemptions.
- `MAX_FILE_REFERENCES` is 5 in the sessions domain. This change affects how candidates are found, not how many can be attached.
- Prompt injection is unchanged: `compose_prompt` remains the only place that reads referenced file content, and it keeps inlining full file text.

## Goals / Non-Goals

**Goals:**

- Give chat mention a candidate source whose eligibility rules are its own.
- Keep the result set small and ordered by usefulness rather than by directory walk order.
- Keep both runtimes working through one service-interface method.
- Leave `ChatInputBox.tsx` with headroom for the follow-up changes.

**Non-Goals:**

- Changing `list_session_documents`, `collect_documents`, or Documents tab behavior in any way.
- Changing prompt injection, persistence, or the file-reference count limit.
- Full-text/content search. This searches paths and filenames only.
- Honoring `.gitignore`. See Decisions.

## Decisions

### A new command rather than a parameter on the existing one

`list_session_documents` could take a `mention_mode` flag that swaps the extension set and enables exclusions. Rejected: one command whose result shape means two different things is exactly the coupling that produced this bug. A separate command lets each consumer's bounds move independently, and it keeps the Documents tab's traversal untouched so its requirement needs no re-verification.

### Query-driven search rather than a wider full listing

A `list_mention_candidates` variant returning every eligible file would still be truncated by a global cap, and a source tree exceeds any reasonable cap long before the interesting file appears. Passing the query to the native layer lets the cap apply to *matches* instead of to *traversal order*, which is what makes the result set useful. The cost is one native call per query instead of one per session.

### Ranking

Scoring is applied to the filename first, then the path, with a fixed tier per match kind:

| Match | Score |
|---|---|
| Filename equals the query (case-insensitive) | 100 |
| Filename starts with the query | 80 |
| Filename contains the query | 60 |
| Query segments all appear in order across the relative path | 40 |
| No match | excluded |

Ties break on shallower path depth first, then on case-insensitive path order, so results are deterministic and the same query always produces the same list. A query containing a path separator is matched against the relative path rather than the filename alone, so `chat/input` finds `src/components/chat/ChatInputBox.tsx`.

Alternative considered: fuzzy subsequence scoring in the style of an editor's quick-open. Rejected for this change - it needs tuning against real repositories to avoid noise, and the tiered rules already cover the reported failure. It remains an option later without changing the spec, which only fixes the relative order of the tiers.

### Extension allowlist rather than a binary-detection denylist

Admitting anything that is not detectably binary would cover more files but pulls in lockfiles, minified bundles, and generated artifacts that are never worth referencing. An allowlist of common source and configuration extensions is maintained in the native layer, extended with a small set of extensionless filenames that matter in practice (`Dockerfile`, `Makefile`, and similar).

Trade-off: the allowlist will miss extensions nobody anticipated, and each gap looks to the user exactly like the bug being fixed here. Mitigated by keeping the list in one named constant next to the exclusion list so it is cheap to extend, and by covering it with a test that asserts representative extensions per language family.

### Directory exclusions as a static list, not `.gitignore`

Parsing `.gitignore` would track each project's real layout, but it requires pattern-matching semantics, nested ignore files, and negation handling, and it fails on sessions whose root is not a Git repository. A static list of well-known vendored and generated directory names is enough to reclaim the result budget. The list is a named constant in the workspaces context so it is greppable and testable.

Note this is deliberately narrower than the traversal exclusion in the existing code: dot-prefixed entries stay excluded as they are today, and the new list adds the non-dot cases (dependency installs, build outputs, compiler caches, virtual environments).

### Runtime boundary

```
  ChatInputBox / composer mention hook
            │  (no invoke(), no direct native access)
            ▼
     agent-service.ts        ← one new method on the service interface
            │
     ┌──────┴──────┐
     ▼             ▼
tauri-agent-   web-agent-
  client        client
     │             │
     ▼             ▼
 invoke(...)   in-memory mock walk over the mock workspace,
     │         same contract, same ranking order
     ▼
 commands/workspaces/<new command>.rs
     │
     ▼
 workspaces context: traversal + allowlist + exclusions + scoring
```

Desktop mode performs a real bounded filesystem walk. Browser/mock mode applies the same ranking to the mock workspace fixture so composer behavior and tests are comparable across runtimes. Both are reached through the same service method; no component-level branching on runtime.

### Debounced querying with request ordering

Every keystroke after `@` would otherwise issue a native call. The composer hook debounces before querying and discards responses that arrive out of order, so a slow query for a short prefix cannot overwrite the results of a later, more specific one. Result caps stay small enough that the panel renders in one frame.

### Extracting composer mention state

Mention parsing, candidate querying, debounce, and selection move out of `ChatInputBox.tsx` into a dedicated hook. This is required, not incidental: the component is 112 lines from the cap and the follow-up changes add line-range chips, drop handling, and paste handling. Participant-mention completion moves with it, since both kinds share the same `@` trigger and the spec requires them to stay distinguishable.

## Risks / Trade-offs

**Traversal latency on a large session root** → Depth limit, exclusion list, and an early stop once enough high-tier matches are collected; combined with debounce so a burst of keystrokes issues one walk.

**Allowlist gaps read as the original bug** → Single named constant, per-language-family test coverage, and a documented extension point.

**Static exclusion list mismatches an unusual project layout** → Only affects result budget, never correctness; excluded trees remain reachable through the Files tab. Widening the list is a one-line change.

**Composer tests currently stub the documents listing** → They must move to the new service method. Left unmigrated they would keep passing against a source the composer no longer uses, hiding a regression.

**Users still capped at 5 attached references** → Unchanged by design, but easier to reach now that candidates actually resolve. If the existing error surfaces as a raw domain error rather than localized feedback, that is a separate defect to file, not to fix here.

**Symlink loops and roots outside containment** → Existing traversal already canonicalizes, tracks visited directories, and enforces root containment; the new walk reuses those safeguards rather than reimplementing them.

## Migration Plan

No data migration, no schema change, no persisted-format change. The new command is additive; `list_session_documents` keeps its callers. Rollback is reverting the composer's candidate source to the documents query, which restores today's behavior exactly.
