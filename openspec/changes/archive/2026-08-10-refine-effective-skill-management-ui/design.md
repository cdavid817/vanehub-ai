## Context

See `proposal.md` for the motivation. The effective Skill runtime now supplies enough metadata for type, delivery, layer, origin, trust, availability, compatibility defaults, usage, resources, and shadowed definitions. The current UI renders most of those values directly in each row and expands runtime details inside the row, producing a dense badge wall and vertical layout shifts.

The implementation must remain within the existing React service boundary, use React local state and Tailwind CSS, preserve both Tauri and Web/mock behavior, reuse the existing semantic theme and Lucide icon family, and keep every production TypeScript or TSX file within the 300-line limit.

## Goals / Non-Goals

**Goals:**

- Establish a stable master-detail interaction in which rows are optimized for scanning and a detail surface is optimized for inspection.
- Preserve the current effective Skill semantics and every existing mutation path.
- Make precedence, immutable state, unsupported Utility state, usage, and resources understandable without expanding every row.
- Provide equivalent detail content on wide and narrow settings viewports with explicit focus behavior.
- Keep the component split small enough for isolated tests and the repository line limit.

**Non-Goals:**

- Changing Skill discovery, precedence, loading, usage counting, assignment, preview, or persistence behavior.
- Adding Overlay, self-evolution, candidate review, or Curator controls to the Skill management page.
- Adding a UI framework, state library, font, animation library, or new service contract.
- Replacing the existing application-wide color tokens or typography.

## Decisions

### 1. Use a master-detail model instead of per-row disclosure

Each row exposes an explicit Details action. The page owns the selected canonical Skill identity and passes selected state to the list. On a sufficiently wide content region, the detail surface renders as a labeled `aside` beside the inventory. On a narrow region, the same presentational body renders inside a focus-managed application panel or sheet.

This keeps a single selected Skill, prevents multiple expanded rows from shifting the list, and preserves the user's filter and Agent context. Keeping the existing row disclosure was considered, but it repeats dense content and makes comparison and keyboard navigation harder.

Conceptual wide layout:

```text
┌──────────────┬──────────────────────────────────┬──────────────────────┐
│ Agent views  │ Filters and Skill inventory      │ Skill details        │
│              │                                  │                      │
│ All          │ [Role] [User] Skill name         │ Identity + state     │
│ Claude       │ Description          [Details]   │ Runtime facts        │
│ Codex        │                                  │ Precedence timeline  │
│ Unassigned   │ [Utility] Skill name             │ Usage + resources    │
│              │ Description    [Assign] [Details]│ Explanations         │
└──────────────┴──────────────────────────────────┴──────────────────────┘
```

On narrow viewports the inventory remains in document order and the detail surface overlays it as an application panel. Content is not duplicated in the accessibility tree: a small media-query hook chooses one presentation, while a shared detail-body component supplies identical content.

### 2. Define three explicit information levels

The row always shows level-one identity and action information: name, enabled or paused state, effective layer, type, description, and the context-specific primary action. Version and compact usage text may appear as low-emphasis secondary text when space permits, but delivery, origin, trust, compatibility, resources, and shadowed definitions move to the inspector.

The inspector groups level-two runtime facts and level-three provenance details under sequential headings. Badges are reserved for short categorical states; facts use definition lists, explanations use text with an icon, and usage uses tabular numbers rather than additional badges.

This preserves access to every existing field while reducing competing visual emphasis. Merely shrinking every badge was considered, but it would retain the same cognitive load and poor responsive behavior.

### 3. Keep mutation hierarchy contextual

In All Skills and Unassigned views, enablement remains the primary mutable control, with Preview, Edit, and Delete shown only where allowed. In selected-Agent views, Assign or Remove remains the only primary button. Details and Preview are secondary controls, and edit/delete/global enablement remain absent.

System Skills use the existing immutable flag to show a compact lock/read-only treatment and omit edit/delete. Unsupported Utility Skills show a concise warning and no Role assignment action. Full explanations live in the inspector. Pending state and operation errors remain keyed to canonical Skill id so unrelated rows remain usable.

### 4. Present precedence as a semantic ordered timeline

The inspector derives a display-only sequence from the effective definition followed by `shadowedDefinitions` in the precedence order supplied by the runtime contract. Every timeline item contains layer, origin, version, availability, and an explicit Effective or Shadowed label. The timeline does not introduce mutation controls or infer missing source paths.

An unordered card grid was considered, but it does not communicate why one definition wins. The ordered timeline makes the precedence relationship visible without adding another table or requiring horizontal space.

### 5. Keep selection local and reconcile it against visible data

`SkillsPage` owns a selected Skill key composed from the canonical identity already used by list rendering. Selection changes only through an explicit Details action. An effect reconciles the key with the filtered inventory and clears it when the Skill disappears after a filter, view change, or overview refresh. Changing filters or opening details never mutates backend state.

The narrow presentation also stores the originating trigger element so the existing focus restoration pattern can return keyboard focus. No global context or external state store is needed.

### 6. Split UI responsibilities along testable boundaries

The current row component is divided into bounded pieces:

- a compact row summary shared by lifecycle and Agent rows;
- row action groups specific to lifecycle and Agent contexts;
- a details surface that chooses wide inspector or narrow application panel;
- a pure detail body for runtime facts, precedence, usage, resources, and explanations;
- small formatting helpers for semantic labels and stable identity.

The existing `ApplicationDialog` focus contract is reused for narrow presentation. If a sheet-like visual variant is needed, it is added as an optional styling variant without changing other dialog defaults. React components continue to call only the existing management hook and frontend service methods.

### 7. Use existing visual tokens with restrained motion

The page keeps the established foreground, muted, border, destructive, warning, success, background, and focus-ring tokens in both themes. Lucide outline icons remain the single icon family. No generated palette, marketing typography, glass effect, or scroll animation is introduced.

Selection uses border or surface emphasis plus `aria-expanded` or explicit selected text, never color alone. Transitions are limited to short opacity or chevron changes and use `motion-reduce` variants. Interactive targets remain at least 40 CSS pixels in the desktop settings context and expand to 44 CSS pixels on narrow touch layouts where practical.

### 8. Test behavior at component and browser levels

Vitest covers row hierarchy, immutable and Utility explanations, selection reconciliation, precedence ordering, focus return, and unchanged mutation calls. Playwright covers keyboard-only detail opening and dismissal, a wide list-detail viewport, 375px narrow presentation, no horizontal page overflow, and the selected-Agent primary action hierarchy.

The test fixtures use representative System, User override, unsupported Utility, compatibility-defaulted, and shadowed definitions. Both the Web/mock rendering path and shared React behavior are exercised without adding adapter-specific UI branches.

## Risks / Trade-offs

- [The persistent inspector reduces inventory width on medium desktop windows] → Enable the adjacent inspector only when the content region can preserve row actions; use the narrow application panel otherwise.
- [Conditional wide/narrow presentation can duplicate state or accessibility content] → Keep one selected identity and one shared detail body, and render only one surface at a time.
- [Selection becomes stale after filtering or a mutation refresh] → Reconcile against the effective visible inventory and close stale details deterministically.
- [Moving metadata out of rows can reduce discoverability] → Keep an explicit labeled Details action, retain critical state in the row, and group all removed facts in the first inspector sections.
- [A shared dialog variant could regress other dialogs] → Make any sheet styling opt-in and retain the existing default markup and focus behavior; cover the default with existing dialog tests.
- [Translations increase across five locales] → Add the same semantic key set to every locale and assert missing-key-free rendering in existing UI tests.

## Migration Plan

1. Add component tests for the final row and inspector behavior while preserving existing mutation assertions.
2. Introduce the shared detail body and responsive surface without changing service contracts.
3. Refactor lifecycle and Agent rows to the new information and action hierarchy.
4. Replace the row-inline runtime disclosure after equivalent inspector coverage is verified.
5. Add localized strings and Playwright coverage for wide, narrow, keyboard, and overflow behavior.
6. Deploy as a frontend-only change. Rollback consists of restoring the previous row renderer and inline detail disclosure; no stored data or native migration is involved.
