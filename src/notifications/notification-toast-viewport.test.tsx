// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
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
});
