// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import type { AsyncViewState } from "../async/async-view-state";
import { Inspector } from "./Inspector";

function detailState(overrides: Partial<AsyncViewState<React.ReactNode>>): AsyncViewState<React.ReactNode> {
  return { initialLoading: false, refreshing: false, stale: false, ...overrides };
}

describe("Inspector", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows overview content and no pin control when there is no selection", () => {
    render(
      <Inspector
        detail={detailState({})}
        mode="overview"
        onPin={vi.fn()}
        onReturnToOverview={vi.fn()}
        onUnpin={vi.fn()}
        overview={<p>Select something to inspect it here.</p>}
        title="Inspector"
      />,
    );
    expect(screen.getByText("Select something to inspect it here.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Pin" })).toBeNull();
  });

  it("follows the main-area selection and offers Pin, not Unpin", () => {
    const onPin = vi.fn();
    render(
      <Inspector
        detail={detailState({ data: <p>Session detail</p> })}
        mode="follow"
        onPin={onPin}
        onReturnToOverview={vi.fn()}
        onUnpin={vi.fn()}
        overview={<p>Overview</p>}
        selectionSummary="Session: fix-flaky-test"
        title="Inspector"
      />,
    );
    expect(screen.getByText("Session detail")).toBeTruthy();
    expect(screen.getByText("Session: fix-flaky-test")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Pin" }));
    expect(onPin).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: "Unpin" })).toBeNull();
  });

  it("keeps a pinned selection through Unpin instead of Pin", () => {
    const onUnpin = vi.fn();
    render(
      <Inspector
        detail={detailState({ data: <p>Session detail</p> })}
        mode="pinned"
        onPin={vi.fn()}
        onReturnToOverview={vi.fn()}
        onUnpin={onUnpin}
        overview={<p>Overview</p>}
        title="Inspector"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Unpin" }));
    expect(onUnpin).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: "Pin" })).toBeNull();
  });

  it("offers a return-to-overview action when the selected object becomes unavailable", () => {
    const onReturnToOverview = vi.fn();
    render(
      <Inspector
        detail={detailState({ error: { kind: "unavailable", message: "gone", retryable: false } })}
        mode="pinned"
        onPin={vi.fn()}
        onReturnToOverview={onReturnToOverview}
        onUnpin={vi.fn()}
        overview={<p>Overview</p>}
        title="Inspector"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Return to overview" }));
    expect(onReturnToOverview).toHaveBeenCalledOnce();
  });

  it("renders a close control only when hosted in a sheet", () => {
    const withoutClose = render(
      <Inspector detail={detailState({})} mode="overview" onPin={vi.fn()} onReturnToOverview={vi.fn()} onUnpin={vi.fn()} overview={<p>Overview</p>} title="Inspector" />,
    );
    expect(withoutClose.queryByRole("button", { name: "Close" })).toBeNull();
    withoutClose.unmount();

    const onClose = vi.fn();
    render(
      <Inspector detail={detailState({})} mode="overview" onClose={onClose} onPin={vi.fn()} onReturnToOverview={vi.fn()} onUnpin={vi.fn()} overview={<p>Overview</p>} title="Inspector" />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("gives the return-to-overview action a visible focus ring, matching every other interactive src/ui/ control", () => {
    render(
      <Inspector
        detail={detailState({ error: { kind: "unavailable", message: "gone", retryable: false } })}
        mode="pinned"
        onPin={vi.fn()}
        onReturnToOverview={vi.fn()}
        onUnpin={vi.fn()}
        overview={<p>Overview</p>}
        title="Inspector"
      />,
    );
    expect(screen.getByRole("button", { name: "Return to overview" }).className).toContain("ucd-focus-ring");
  });

  it("surfaces a retryable detail-load error through AsyncBoundary's retry action", () => {
    const onRetryDetail = vi.fn();
    render(
      <Inspector
        detail={detailState({ error: { kind: "error", message: "Could not load detail.", retryable: true } })}
        mode="follow"
        onPin={vi.fn()}
        onReturnToOverview={vi.fn()}
        onRetryDetail={onRetryDetail}
        onUnpin={vi.fn()}
        overview={<p>Overview</p>}
        title="Inspector"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(onRetryDetail).toHaveBeenCalledOnce();
  });
});
