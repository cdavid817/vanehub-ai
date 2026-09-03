// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { getActivePendingTimerCount } from "../testing/resource-tracking";
import { createNotificationRecord, notificationReducer, VISIBLE_TOAST_LIMIT } from "./notification-reducer";
import { NotificationToastViewport } from "./notification-toast-viewport";
import type { NotificationRecord } from "./notification-types";

const record = (id: string, overrides: Partial<NotificationRecord> = {}): NotificationRecord => ({
  id,
  type: "success",
  title: `通知 ${id}`,
  message: null,
  scope: { kind: "global" },
  createdAt: "2026-01-01T00:00:00Z",
  read: false,
  toastState: "visible",
  durationMs: 5_000,
  ...overrides,
} as NotificationRecord);

describe("NotificationToastViewport", () => {
  afterEach(() => vi.useRealTimers());

  it("anchors the toast band to the top of the viewport", () => {
    render(
      <NotificationToastViewport
        activeSessionId={null}
        notifications={[record("1")]}
        onBeginToastExit={vi.fn()}
        onHideToast={vi.fn()}
      />,
    );

    const region = screen.getByLabelText("通知动态");
    // Bottom left covered the session list, bottom right sits on the composer's send button and
    // top right sits on the information panel tabs. The top band covers none of the three, and
    // top-12 clears the top bar rather than sitting on its search and focus controls.
    expect(region.className).toContain("top-12");
    expect(region.className).not.toContain("bottom-");
    expect(region.className).toContain("sm:left-1/2");
  });

  it("stacks toasts newest-first without hiding any of them", () => {
    render(
      <NotificationToastViewport
        activeSessionId={null}
        notifications={[record("1"), record("2")]}
        onBeginToastExit={vi.fn()}
        onHideToast={vi.fn()}
      />,
    );

    const titles = screen.getAllByRole("status").map((toast) => toast.textContent);
    expect(titles).toHaveLength(2);
    expect(titles[0]).toContain("通知 2");
  });

  /**
   * Task 21.16's "global attention coordinator" half: `NotificationHost`/`NotificationProvider`
   * (`src/notifications/notification-provider.tsx`) is real and genuinely global -- mounted once at
   * `App.tsx`'s top level, well above `MainLayout`'s route outlet (confirmed by reading both), so it
   * never unmounts on a destination switch the way the 7 ordinary destinations do. Unlike them, it
   * legitimately does NOT release its resources on navigation; the property worth proving instead is
   * that it stays *bounded* regardless of how many notifications ever fire. `GlobalAttentionSummary`
   * (the name design.md's own diagram uses) is not this -- 5.1's own evidence already found it was
   * never built, and that remains true; this tests the real mechanism that exists under a different
   * name, not a stand-in for the fictional one.
   *
   * `VISIBLE_TOAST_LIMIT` (notification-reducer.ts) is the real, already-existing budget --
   * `notification-reducer.test.tsx` already proves the *reducer state* stays capped at it, but
   * never that the live `setTimeout` auto-dismiss timers those visible toasts each arm stay capped
   * too, nor that they are fully released rather than accumulating one per notification ever fired.
   */
  it("never arms more than VISIBLE_TOAST_LIMIT auto-dismiss timers even when a burst publishes far more notifications", () => {
    vi.useFakeTimers();
    let state: NotificationRecord[] = [];
    for (let index = 0; index < 10; index += 1) {
      state = notificationReducer(state, {
        type: "published",
        notification: createNotificationRecord({ type: "info", title: `burst ${index}` }, `burst-${index}`, index),
      });
    }
    // The reducer-state half of the budget (already covered by notification-reducer.test.tsx),
    // re-asserted here only to pin the fixture's own shape before the new claim below.
    expect(state.filter((item) => item.toastState !== "hidden")).toHaveLength(VISIBLE_TOAST_LIMIT);

    render(
      <NotificationToastViewport activeSessionId={null} notifications={state} onBeginToastExit={vi.fn()} onHideToast={vi.fn()} />,
    );

    // 10 notifications were published; at most VISIBLE_TOAST_LIMIT ever have a live timer, not 10.
    expect(getActivePendingTimerCount()).toBe(VISIBLE_TOAST_LIMIT);
  });

  it("clears every pending auto-dismiss timer on unmount instead of leaking one per toast", () => {
    vi.useFakeTimers();
    const state = Array.from({ length: VISIBLE_TOAST_LIMIT }, (_unused, index) => record(`visible-${index}`));

    const { unmount } = render(
      <NotificationToastViewport activeSessionId={null} notifications={state} onBeginToastExit={vi.fn()} onHideToast={vi.fn()} />,
    );
    expect(getActivePendingTimerCount()).toBe(VISIBLE_TOAST_LIMIT);

    unmount();
    expect(getActivePendingTimerCount()).toBe(0);
  });
});
