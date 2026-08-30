# Visual-token hard-coding audit (task 2.1)

Scanned `main-layout/`, `session-workspace/`, `components/chat/`, `settings/`, `work-board/`,
`goal-center/`, `mission-control/`, `loop-center/`, `evaluation-center/`, and
`main-layout/scheduled-tasks-dialog.tsx` (no separate `src/scheduled-tasks/` exists) for hard-coded
colors, non-standard radii, control heights, ad-hoc shadows, and theme-branching JSX.

## Prior art this audit changed my plan around

- **`session-workspace/console-visual-tokens.test.ts` + `visual-token-rules.ts`** already enforce
  zero literal colors (`LITERAL_COLOR`) and zero Tailwind default-palette classes (`PALETTE_COLOR`)
  across all of `main-layout/` and `session-workspace/`. Task 2.12's new `ARCH-FE-006` architecture
  guard (scoped to `src/ui/`) now **imports these same two patterns** rather than re-deriving them,
  so the two guards can't drift apart — Node 24's built-in TypeScript support loads the `.ts` file
  directly from the `.mjs` architecture script with no build step.
- **`dark:` Tailwind variants are dead code app-wide.** Theming is 100% via
  `document.documentElement.dataset.theme`, never a `.dark` class — every `dark:*` utility found
  (6 locations, all in `settings/`/`goal-center/`, none in the two console-guarded directories)
  never activates. Not fixed here (out of §2's scope — these are pre-existing files this milestone
  doesn't touch); flagged for whoever migrates those specific files (`settings/pages/agents/
  cli-config-profile-list.tsx`, `settings/pages/skills/skill-row-summary.tsx`,
  `goal-center/goal-presentation.ts`, `goal-center/goal-detail.tsx`) in a later milestone.
- **Theme-branching JSX: zero found** (task 2.8's actual audit target). Every theme-identity read
  in these 9 directories only changes CSS custom-property *values* or plain text — never component
  structure. Task 2.8 required no code change; this file is its evidence.

## Real hard-coded chrome colors (in scope for future token migration)

Two, both outside the console-guarded directories:
- `settings/pages/mcp/mcp-server-card.tsx:40` — `bg-[#22c55e]` / `bg-[#94a3b8]` for the active/
  inactive status dot. Should become `bg-success` / `bg-neutral` when this page is migrated.
- `settings/pages/local-media/compatibility-notice.tsx:61`,
  `settings/pages/agents/lsp-workspace-trust-panel.tsx:55` — `border-amber-500/40 bg-amber-500/10`
  pattern (Tailwind default palette, not a token). Candidates for `warning`/`attention`.
- `settings/pages/agents/cli-config-profile-list.tsx:10-13`, `skills/skill-row-summary.tsx:37`,
  `goal-center/goal-presentation.ts:39-40`, `goal-center/goal-detail.tsx:79,97` — more Tailwind
  default-palette colors (`blue-500`, `violet-500`, `emerald-500`, `amber-*`), each paired with a
  dead `dark:` variant (see above).

**Explicitly out of scope**: `settings/pages/expert-roles/expert-role-form.tsx:8` (6-swatch color
picker for user-assigned role colors) and `components/chat/MessageItem.tsx:69`
(`speaker.color` flowing into an SVG `fill`) are *user/domain data*, not UI chrome — they represent
colors a user chose, not a theme-driven surface. These correctly stay literal/dynamic.

## Existing tokens with no `@theme` mapping — fixed in this milestone

`--nav-active-soft` (≈30 call sites — the single most common arbitrary-color pattern in the audited
surface), `--panel-glass`, `--panel-hover`, `--danger-soft`, `--success-soft`, `--warning-soft` had
real per-theme values but no generated Tailwind utility class, forcing arbitrary syntax
(`bg-[hsl(var(--nav-active-soft))]`) at every call site. Added `--color-nav-active-soft` etc. to
`styles.css`'s `@theme` block — purely additive, no existing call site changed, but new code can
now write `bg-nav-active-soft`.

**Not fixed here** (real but disproportionate to this milestone): ~50+ call sites already use
`bg-[hsl(var(--panel-muted))]` even though `bg-panel-muted` already existed before this change —
migrating those is a mechanical but large-volume cleanup better done incrementally as each file is
touched for its own feature migration, not as a standalone sweep.

## Non-standard radius

`rounded-xl` (12px) appears at 10 sites in `settings/`, including inside the shared `SectionPanel`
component's `variant="settings"` branch (`settings/pages/page-parts.tsx:75`) — every settings page
passing that variant inherits 12px radius, inconsistent with both `AGENTS.md`'s 8px ceiling and
`SectionPanel`'s own `variant="card"` branch (`rounded-lg`, correct). **Not fixed in this
milestone** — deliberately deferred to task group 12 (Settings), where `SectionPanel` itself gets
migrated to the new shared primitives; fixing the radius in isolation now would create a second,
disconnected visual change shortly before the real migration replaces the component anyway.

`rounded-2xl`/`rounded-3xl`/arbitrary `rounded-[...]`: none found. `rounded-full`: extensive but
every instance is a pill/chip/toggle-track or a genuinely circular icon/status-dot container — no
instance applied to a panel/card/dialog surface.

## Control height evidence (corrected `--control-height-*` tokens against this data)

Real inventory: h-8 (32px) and **h-9 (36px, by far the most common — 111 occurrences in `settings/`
alone)** are the two dominant heights, driven by `components/ui/button.tsx`'s `default`/`sm`
variants. h-11 (44px) is already a deliberate, existing touch-target tier at narrow widths (paired
as `h-11 ... sm:h-9` in several settings surfaces). Zero arbitrary fixed-pixel control heights
(`h-[Npx]`) found anywhere. Based on this, `--control-height-default` is **36px** (not the
disconnected 32px this milestone's tokens initially guessed before the audit landed) and
`--control-height-comfortable` is **44px** (matching the real existing touch-target tier), keeping
`--control-height-compact` at 28px (close to the existing h-7 compact tier). `--row-height-*`
tokens are unrevised — this audit's control-height focus didn't cover list-row heights specifically
and found no contradicting evidence for the original 32/40/48 scale.

## Ad-hoc shadows

`shadow-xs`/`shadow-sm`/`shadow-lg`/`shadow-xl` all appear directly on elements beyond
`.ucd-panel`/`.ucd-card`, without a clear tier convention (e.g. `shadow-lg` is used for dropdown-
style overlays in some files, `shadow-xl` for similar overlays in others). The new
`--shadow-overlay` tier (§2.4) gives modal/sheet-level content a documented home; reconciling the
existing ad-hoc usages into the 3-tier scale (`shadow-color` / `shadow-elevated` / `shadow-overlay`)
is deferred to each surface's own migration, not swept here.

## Deferred to §3 (needs primitives that don't exist yet)

- **Task 2.10** (nested-card pilot): no clear-cut nested-card-in-card instance was found in the 9
  audited directories using only the *existing* `page-parts.tsx`/`Card` primitives — the one real,
  well-evidenced finding (`SectionPanel`'s `rounded-xl` inconsistency, above) is a radius bug, not
  a nested-decoration pattern. Doing a "pilot" now with soon-to-be-replaced legacy primitives would
  produce a throwaway example. Deferred to the first real page migration in task group 12, where it
  can use the actual new `src/ui/` primitives being piloted.
- **Task 2.11** (visual token/state fixture page): needs `AsyncBoundary`/`StatusBadge`/etc. (§3) to
  meaningfully demonstrate interactive/loading/disabled/selected/error states — a fixture page with
  only raw color swatches would be a weak deliverable. Deferred to land alongside §3's primitives.
