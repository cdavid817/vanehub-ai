import { DEFAULT_PAGE_LIFECYCLE_POLICY, type PageLifecyclePolicy } from "../ui/page-lifecycle/page-lifecycle-policy";
import type { WorkbenchDestination } from "./workbench-route";

/**
 * design.md Decision 6, audited per destination rather than assumed — the top-level counterpart to
 * settings-page-lifecycle.ts. Unlike Settings, this does not drive a runtime mount/unmount decision:
 * `main-layout.tsx`'s workbench-route-outlet already renders `projects`/`runs`/`plan`/`quality` as
 * `{location.destination === "x" ? <XDestination/> : null}` with no `key`, which per React's
 * reconciliation rules fully unmounts the previous destination on every switch — that already *is*
 * `keepAlive: "never"`, with nothing left to build. This table exists so that fact is a documented,
 * typed decision instead of an accident of how the ternary happened to be written.
 */
export const DESTINATION_LIFECYCLE: Record<WorkbenchDestination, PageLifecyclePolicy> = {
  // The one exception: main-layout.tsx renders Sessions' DestinationLayout unconditionally and
  // toggles only a CSS `hidden` class, so it is never torn down once mounted. `draft-only`, not
  // `always`: the one concrete, tested reason is the composer's in-progress draft
  // (tests/e2e/workspace-routing.spec.ts, "preserves session state, including an in-progress
  // draft..."), which fits Decision 6's own `draft-only` definition ("仅含未提交草稿且难以序列化的页
  // 面") — no live connection whose reconnect cost matters was found to justify `always` instead
  // (active-session/agent state is backend-owned per AGENTS.md's architecture rule, not something
  // Sessions' own mount provides).
  sessions: { keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: true, backgroundUpdates: "none" },
  projects: DEFAULT_PAGE_LIFECYCLE_POLICY,
  runs: DEFAULT_PAGE_LIFECYCLE_POLICY,
  plan: DEFAULT_PAGE_LIFECYCLE_POLICY,
  quality: DEFAULT_PAGE_LIFECYCLE_POLICY,
};
