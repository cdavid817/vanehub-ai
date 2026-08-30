import { describe, expect, it } from "vitest";
import { DEFAULT_PAGE_LIFECYCLE_POLICY, shouldRenderPage, type PageLifecyclePolicy } from "./page-lifecycle-policy";

const draftOnly: PageLifecyclePolicy = { keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: true, backgroundUpdates: "none" };
const always: PageLifecyclePolicy = { keepAlive: "always", suspendWhenHidden: false, refreshOnFocus: true, backgroundUpdates: "terminal-only" };

describe("shouldRenderPage", () => {
  it("always renders the active page, regardless of policy or visit history", () => {
    expect(shouldRenderPage(DEFAULT_PAGE_LIFECYCLE_POLICY, true, false)).toBe(true);
    expect(shouldRenderPage(draftOnly, true, false)).toBe(true);
    expect(shouldRenderPage(always, true, true)).toBe(true);
  });

  it("never renders a never-visited inactive page, regardless of policy", () => {
    expect(shouldRenderPage(DEFAULT_PAGE_LIFECYCLE_POLICY, false, false)).toBe(false);
    expect(shouldRenderPage(draftOnly, false, false)).toBe(false);
    expect(shouldRenderPage(always, false, false)).toBe(false);
  });

  it("does not keep a never-policy page mounted once inactive, even if previously visited", () => {
    expect(shouldRenderPage(DEFAULT_PAGE_LIFECYCLE_POLICY, false, true)).toBe(false);
  });

  it("keeps a draft-only or always page mounted while inactive, once visited", () => {
    expect(shouldRenderPage(draftOnly, false, true)).toBe(true);
    expect(shouldRenderPage(always, false, true)).toBe(true);
  });
});
