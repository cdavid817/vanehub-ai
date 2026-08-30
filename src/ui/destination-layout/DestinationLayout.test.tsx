// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { DestinationLayout } from "./DestinationLayout";

describe("DestinationLayout", () => {
  beforeAll(() => {
    // jsdom does not implement ResizeObserver; this repo's convention (shell-tab.test.tsx) is a
    // no-op stub, which leaves a mounted layout at its default tier — tier-specific composition
    // is covered directly against DestinationLayoutBody instead.
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  });

  it("renders the work surface and observes its container for width changes", () => {
    const observe = vi.spyOn(ResizeObserver.prototype, "observe");
    const { unmount } = render(<DestinationLayout main={<main>Work surface</main>} />);
    expect(screen.getByText("Work surface")).toBeTruthy();
    expect(observe).toHaveBeenCalledOnce();

    const disconnect = vi.spyOn(ResizeObserver.prototype, "disconnect");
    unmount();
    expect(disconnect).toHaveBeenCalledOnce();
  });
});
