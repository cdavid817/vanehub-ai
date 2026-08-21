## Context

See `proposal.md` for motivation. The current settings page renders summary cards, five simultaneous filters, two-column cards with per-Hook CLI checkboxes, and a trace table in one document flow. User Hooks also have separate edit and advanced dialogs even though both mutate the same draft lifecycle.

The existing Prompt Hook service contract already supports listing, enablement, binding, preview, draft persistence, publication, rollback, version history, evaluation, and safe traces in both Tauri and Web/mock adapters. This change should reorganize those capabilities without widening the runtime boundary.

## Goals / Non-Goals

**Goals:**

- Make enablement, status inspection, and Hook discovery efficient from the first viewport.
- Give each user Hook one comprehensible path from configuration through draft publication and history.
- Preserve advanced governance and diagnostics through progressive disclosure.
- Preserve accessibility, responsive behavior, localization, and measured virtualization for large inventories.

**Non-Goals:**

- Changing Prompt assembly semantics, Hook manifests, governance rules, persistence, or evaluation calculations.
- Adding bulk mutation, drag-and-drop ordering, server-side filtering, or a new backend endpoint.
- Replacing the existing service interface, query library, shared UI primitives, or Tailwind styling system.

## Decisions

### 1. Divide the page into management and runtime-records views

The page will expose two semantic views: **Hook management** and **Runtime records**. Management is the default and contains inventory operations; runtime records owns assembled-prompt preview and safe trace summaries. The active view remains frontend presentation state because both views belong to one settings destination and do not need a new application route.

This separates configuration from diagnosis while preserving quick access. Keeping the trace table under the inventory was rejected because it makes an already long page grow with unrelated operational data. Creating a separate settings navigation item was rejected because traces remain subordinate to Prompt Hooks.

Trace data should be queried when runtime records is first opened, while already loaded data remains visible during refresh. Hook and agent queries remain shared at the page level so preview and detail surfaces use the same stable agent-id inputs in both runtimes.

### 2. Replace large cards with category-grouped compact rows

The management inventory will render localized category headers with counts and compact Hook rows. A row exposes identity, source, enabled state, publication state, CLI-binding count, and only the actions needed to enter detail or toggle state. Full binding checkboxes, governance metadata, hash, and token estimates move out of the default row.

Category grouping makes the seven-category model visible without requiring category to consume permanent toolbar space. Groups are expanded by default when they contain search results; explicit user expansion state is preserved until the filter result changes. Stable execution order is retained inside each category.

A compact table was considered but rejected because narrow settings layouts and variable-length descriptions need responsive wrapping. Compact rows preserve document semantics without requiring horizontal scrolling.

### 3. Use progressive filters with visible active state

Search, enabled state, and stable CLI binding remain in the primary toolbar. Source, stage, and explicit category filtering move to an additional-filter popover or collapsible panel. Active additional filters produce a visible count or chips and a single clear action.

This preserves every existing filter dimension while reducing first-view choice load. Removing lower-frequency filters entirely was rejected because existing workflows and specifications require them.

### 4. Use one responsive Hook detail surface

Selecting a row opens one accessible detail surface. On wide layouts it may appear as a right-aligned panel; on narrow layouts it becomes a bounded full-width application dialog. Both presentations share the same content and focus-management contract rather than creating separate desktop and mobile workflows.

The detail surface organizes content into:

1. **Overview**: identity, category, stage, order, governance state, enablement, and CLI bindings.
2. **Content & publication**: template-variable guidance, template draft, preview, save-draft action, published version, and publish action.
3. **Version history**: immutable versions, operational evaluations, and confirmed rollback.

Built-in Hooks expose read-only overview, allowed enablement and bindings, and preview, but omit draft, publish, and rollback controls. User Hooks expose all three sections. Create uses the same field vocabulary but starts in a focused creation state; destructive deletion remains confirmed.

Keeping separate edit and advanced dialogs was rejected because the user cannot reliably predict which editor changes the draft or reaches publication.

### 5. Preserve service and runtime boundaries

React continues to call only `AgentService`. The Tauri adapter remains the only frontend layer that may invoke native Prompt Hook commands, the Web/mock adapter retains contract parity, and Rust remains responsible for SQLite and runtime behavior. No service signature or native schema change is expected.

Existing query keys can be retained. Hook histories used for inventory draft indicators may still load per user Hook; detail-only version bodies and variables remain queried on demand. If implementation reveals an unacceptable N+1 cost, batching requires a separate service-contract proposal rather than an implicit UI-only change.

### 6. Adapt virtualization to grouped compact rows

At 500 Hooks or fewer, grouped rows remain in normal document flow. Above the threshold, the virtual input becomes a flattened sequence of category headings and Hook rows. Each item has a stable typed key, measured height, accessible position metadata where applicable, and at most four overscan rows on either side.

This retains the existing scalability requirement while avoiding two-column regrouping. Virtualizing independently inside each group was rejected because nested scroll regions and multiple virtualizers complicate focus and measurement.

## Risks / Trade-offs

- [Users may overlook diagnostics after traces leave the default document flow] → Use a clearly labeled runtime-records view and surface failed or recent trace state in its navigation label when available.
- [A unified detail surface can itself become dense] → Separate overview, content/publication, and history into explicit sections or tabs and keep the primary state/action area persistent.
- [Category collapsing can hide search matches] → Automatically expand groups containing matches when search or filters change and announce result counts.
- [Flattened grouped virtualization adds row-type complexity] → Define one typed view model and test headings, Hook positions, responsive measurement, filtering, and offscreen interaction.
- [Moving trace loading behind a view changes request timing] → Keep service behavior unchanged and test both first-load and refresh-with-previous-data states.

## Migration Plan

1. Add the new localized view, summary, filter, group, detail, and action labels.
2. Introduce compact grouped inventory presentation while retaining current mutations and query keys.
3. Consolidate edit and lifecycle content into the unified detail workflow.
4. Move assembly preview and trace presentation into runtime records and make trace loading view-aware.
5. Update component, interaction, accessibility, virtualization, and Playwright coverage.

The change has no persisted-data migration. Rollback consists of restoring the prior page composition; service data and published Hook versions remain compatible.
