// @vitest-environment jsdom

import { fireEvent, screen, waitFor } from "@testing-library/dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createBootstrapFailureEvent,
  recoverFromBootstrapFailure,
  type BootstrapFailureCopy,
} from "./bootstrap-failure";

const copy: BootstrapFailureCopy = {
  title: "VaneHub failed to start",
  description: "The application surface could not load.",
  retry: "Reload",
};

describe("frontend bootstrap recovery", () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="root"></div>';
  });

  it.each(["main", "floating-assistant"] as const)(
    "renders a visible retry surface when %s bootstrap rejects",
    async (surface) => {
      const retry = vi.fn();
      const report = vi.fn().mockResolvedValue(undefined);

      recoverFromBootstrapFailure({
        root: document.getElementById("root") as HTMLElement,
        copy,
        error: new Error("chunk unavailable"),
        surface,
        retry,
        report,
      });

      expect(document.querySelector('[data-bootstrap-recovery="true"]')).not.toBeNull();
      expect(screen.getByRole("heading", { name: copy.title })).not.toBeNull();
      fireEvent.click(screen.getByRole("button", { name: copy.retry }));
      expect(retry).toHaveBeenCalledOnce();
      await waitFor(() => expect(report).toHaveBeenCalledWith(expect.objectContaining({
        kind: "critical-operation-failure",
        source: "frontend-bootstrap",
        details: { surface },
      })));
    },
  );

  it("keeps recovery visible when diagnostic reporting fails", async () => {
    const report = vi.fn().mockRejectedValue(new Error("runtime adapter unavailable"));

    recoverFromBootstrapFailure({
      root: document.getElementById("root") as HTMLElement,
      copy,
      error: "bootstrap rejected",
      surface: "main",
      retry: vi.fn(),
      report,
    });

    await waitFor(() => expect(report).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: copy.retry })).not.toBeNull();
    expect(document.getElementById("root")?.textContent).not.toBe("");
  });

  it("normalizes non-Error rejections for unified client logging", () => {
    expect(createBootstrapFailureEvent("module rejected", "main")).toEqual({
      level: "error",
      kind: "critical-operation-failure",
      message: "module rejected",
      source: "frontend-bootstrap",
      details: { surface: "main" },
      stack: undefined,
    });
  });
});
