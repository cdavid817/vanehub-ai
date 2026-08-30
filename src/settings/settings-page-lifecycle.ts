import { DEFAULT_PAGE_LIFECYCLE_POLICY, type PageLifecyclePolicy } from "../ui/page-lifecycle/page-lifecycle-policy";
import type { SettingsPageId } from "./settings-page-types";

/**
 * design.md Decision 6, audited per page rather than assumed: every settings page used to stay
 * mounted forever once visited (`SettingsPageContext.isActive` existed only to gate background
 * work under that always-mounted contract). Two pages have a documented, in-memory-only draft a
 * reader would be annoyed to lose — those are the only `draft-only` entries; everything else
 * defaults to `never` per Decision 6's own default, restated explicitly here rather than left
 * implicit.
 */
export const SETTINGS_PAGE_LIFECYCLE: Record<SettingsPageId, PageLifecyclePolicy> = {
  basic: DEFAULT_PAGE_LIFECYCLE_POLICY,
  "agent-configurations": DEFAULT_PAGE_LIFECYCLE_POLICY,
  "agent-policies": DEFAULT_PAGE_LIFECYCLE_POLICY,
  // `cli-parameters/draft-state.ts` + `use-cli-parameter-drafts.ts`: a dedicated, tested,
  // in-memory-only draft layer for edits not yet applied to any CLI's real config.
  "cli-parameters": { keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: true, backgroundUpdates: "none" },
  "code-intelligence": DEFAULT_PAGE_LIFECYCLE_POLICY,
  mcp: DEFAULT_PAGE_LIFECYCLE_POLICY,
  skills: DEFAULT_PAGE_LIFECYCLE_POLICY,
  // `instruction-drafts.ts`: `InstructionDraftMap` with `isDirty`/`editDraft`/`discardDraft` across
  // potentially several personalization instructions at once, in memory only.
  personalization: { keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: true, backgroundUpdates: "none" },
  "prompt-hooks": DEFAULT_PAGE_LIFECYCLE_POLICY,
  "expert-roles": DEFAULT_PAGE_LIFECYCLE_POLICY,
  // `local-media-page.tsx`'s `draft: LocalMediaProfile | null` is explicitly documented in that
  // file as "never overwritten by a background refresh" — an in-progress engine-setup choice with
  // no server-side counterpart to reload from.
  "local-media": { keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: true, backgroundUpdates: "none" },
  providers: DEFAULT_PAGE_LIFECYCLE_POLICY,
  extensions: DEFAULT_PAGE_LIFECYCLE_POLICY,
  plugins: DEFAULT_PAGE_LIFECYCLE_POLICY,
  im: DEFAULT_PAGE_LIFECYCLE_POLICY,
  "ssh-connections": DEFAULT_PAGE_LIFECYCLE_POLICY,
  observability: DEFAULT_PAGE_LIFECYCLE_POLICY,
  // Already does exactly what Decision 6 asks on its own (`usageRefetchInterval(isActive)` disables
  // its polling while inactive) — `never` just means the whole page unmounts instead of idling.
  usage: DEFAULT_PAGE_LIFECYCLE_POLICY,
  help: DEFAULT_PAGE_LIFECYCLE_POLICY,
  about: DEFAULT_PAGE_LIFECYCLE_POLICY,
};
