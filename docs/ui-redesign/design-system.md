# Unified workbench design system reference

Tasks 2.4/2.9/2.13/2.14 of `redesign-unified-workbench-ui`. Documents the additive tokens in
`src/styles.css` (§2.2–§2.7) and the presentation rules new shared UI (`src/ui/`, built in §3
onward) must follow. Nothing here changes an existing token's value — see the "Additive workbench
redesign tokens" comments in `src/styles.css` for exactly what's new versus pre-existing.

## Token reference

All tokens are HSL triplets (optionally with an alpha component) stored as CSS custom properties in
`src/styles.css`, mapped into Tailwind's `@theme` block as `--color-*` so they're usable as ordinary
utility classes (e.g. `bg-canvas`, `text-attention`), and re-declared per theme in `:root`,
`:root[data-theme="futuristic"]`, and `:root[data-theme="minimal"]`.

### Surface tokens (§2.2)

| Token | Aliases | Meaning |
|---|---|---|
| `canvas` | `--background` | The app-wide base surface. Default background for every destination. |
| `sidebar` | `--panel-muted` | Navigation-rail / context-navigation background. |
| `panel` | (pre-existing, unchanged) | The default content-surface tone for cards, panels, rows. |
| `raised` | `--panel` + `--shadow-elevated` | An independently floating surface (popover, dropdown, floating menu) — same material as `panel`, deeper shadow. Not for routine cards; see "Surface hierarchy" below. |
| `overlay` | new | Backdrop scrim behind modals/sheets. Near-black regardless of theme (dimming effect), consumed as `hsl(var(--overlay) / 0.5)` via `.ucd-overlay`. |

### Border tiers (§2.2)

| Token | Meaning |
|---|---|
| `border-subtle` | New, lighter than the default tier — for quiet separators between adjacent same-level regions (e.g. the session-list-to-conversation divider). |
| `border-default` (`--border`) | The existing default border — most component borders. |
| `border-strong` (`--border-strong`, pre-existing) | Emphasis borders — resize handles on hover/focus, selected states. |

### Status token groups (§2.5)

Existing: `success`, `warning`, `danger` (each with a `-soft` background variant). New: `neutral`,
`running`, `information`, `blocked`, `attention` (each also with a `-soft` variant), exposed as
`.ucd-status-<name>` utility classes matching the existing `.ucd-status-success/warning/danger`
pattern. `information` intentionally reuses `primary`'s hue per theme (blue reads as "neutral
informative" in this palette) rather than inventing a competing blue.

Per the workbench-design-system-ui spec's "Express status" requirement, **no status may be
color-only** — every use of a status token must pair it with text, an icon, or a shape; these
classes exist to carry the semantic tone, not to be the state's only signal.

### Control and row heights (§2.3)

| Tier | Control height | Row height |
|---|---:|---:|
| Compact | 28px | 32px |
| Default | 36px | 40px |
| Comfortable | 44px | 48px |

Existing components keep their current heights until they're migrated in a later milestone — these
tokens exist now so new `src/ui/` primitives have a documented scale to build against from the
start, per task 2.3's "without changing existing density defaults until component migration."
Control-height values are grounded in the [token audit](token-audit.md)'s real inventory rather
than guessed: h-9 (36px) is the actual dominant existing "default" height (111 occurrences in
`settings/` alone, driven by `components/ui/button.tsx`), and h-11 (44px) is already a deliberate
touch-target tier at narrow widths — not a new invention.

### Elevation (§2.4)

Three shadow tiers, consumed as `hsl(var(--shadow-*))`: `shadow-color` (resting, pre-existing),
`shadow-elevated` (raised surfaces, pre-existing), `shadow-overlay` (new — modal/sheet-level, the
deepest tier, used behind the highest z-order content).

### Motion (§2.4/§2.7)

`motion-fast` (120ms), `motion-base` (160ms, matches the duration already hard-coded across
existing `.ucd-interactive`/`.ucd-list-row` transitions), `motion-slow` (240ms), consumed as
`var(--motion-*)` inside a `transition` declaration. Under `prefers-reduced-motion: reduce`, all
three collapse to `0ms` globally (see the media query in `styles.css`) — new code should prefer
these tokens over a hard-coded duration specifically so it inherits that behavior for free.

### Radius (§2.4, pre-existing)

`radius-lg` (8px), `radius-md` (6px), `radius-sm` (4px) — unchanged. Per `AGENTS.md`, cards and
panels use 8px or less; nothing in this redesign raises that ceiling.

### Focus rings (§2.6)

`.ucd-focus-ring` — a two-layer halo (canvas-colored gap + colored ring) that stays visible
regardless of the element's own background, since the gap always shows the surface behind it rather
than fighting that surface's color. `.ucd-focus-ring-on-danger` swaps the ring color so a focused
control on a danger-toned surface doesn't rely on a same-hue ring for contrast. The existing global
`*:focus-visible` outline remains the default for everything that doesn't opt into these.

## Surface hierarchy (§2.9)

The default hierarchy is **canvas plus quiet separators** — most of the app is one continuous
canvas-toned surface with `border-subtle`/`border-default` separators between regions, not stacked
cards. `raised` is reserved for content that genuinely floats independently of normal document flow
(a popover anchored to a trigger, a dropdown menu, a floating command palette) — not a general
"make this section stand out" tool. A page section that wants emphasis without floating uses a
plain `panel`/`card` surface with a border, per the existing "avoid nested card-in-card decoration"
rule in `AGENTS.md` — reaching for `raised` for that purpose defeats the tiering this token exists
to express.

## Text hierarchy, truncation, and identifiers (§2.13)

- **Primary content** (a message body, a card title) is the visually dominant text on its row —
  normal weight/size, `panel-foreground`/`foreground` color.
- **Secondary/metadata text** (timestamps, counts, secondary status) uses `muted-foreground` and a
  smaller size; it must never out-compete primary content for attention.
- **Truncation**: single-line text that can overflow (titles, paths, labels) truncates with
  ellipsis and carries the full value in a `title` attribute or accessible description — it must
  never silently clip without a way to recover the full value. Multi-line metadata (e.g. a bounded
  description) wraps up to a documented line-clamp rather than truncating to one line when the
  full text is short enough to matter.
- **Monospace identifiers**: stable ids, paths, branch names, and other non-prose identifiers use a
  monospace font stack so they're visually distinguishable from prose and so similar-looking
  characters (`0`/`O`, `l`/`1`) stay legible. This applies to the kind of literal values `AGENTS.md`
  already exempts from localization (agent ids, model ids, paths, command-like values).
- **Bidirectional safety**: identifiers, paths, and mixed-language labels must not visually invert
  or overlap adjacent controls even though no shipped locale is RTL (task 20.16) — wrap identifiers
  in `dir="ltr"` where they sit inside a locale string that could otherwise apply RTL heuristics to
  a mixed-script value.

## Metadata budget per row/card type (§2.14)

Each of these documents the **maximum** bounded metadata a default (non-expanded) row or card may
show before the rest moves to a tooltip, context menu, or Inspector — matching the row-metadata
rules already stated in the delta specs (`main-layout-ui`'s "Render a session row",
`unified-todo-board`'s "Bounded work-item card metadata").

| Surface | Primary | Bounded secondary line(s) | Everything else goes to |
|---|---|---|---|
| Session row | Agent/role identity + title | One bounded secondary line (state, relative time) | Tooltip, context menu, or Inspector |
| Work Item card | Title + actionable state | Up to three secondary metadata groups | Editor sheet or Inspector |
| Run row | Owner/title + canonical state | Elapsed/attention reason | Run detail |
| Goal row | Title + status/progress | One bounded secondary line | Goal detail |
| Evaluation row | Outcome + Agent/config snapshot + task | Core metrics (bounded) | Result detail / column settings |

A row or card that needs more than this budget to be useful is a signal the extra fields belong in
the owning detail surface (Inspector, editor sheet, or dedicated detail route), not that the budget
should be widened — widening it recreates the "every row shows everything" density problem this
redesign exists to fix.
