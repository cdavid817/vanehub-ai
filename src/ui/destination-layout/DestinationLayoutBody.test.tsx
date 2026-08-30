// @vitest-environment jsdom

import { useEffect } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { HorizontalPaneRegion, RuntimePanelRegion } from "./regions";
import { DestinationLayoutBody } from "./DestinationLayoutBody";

/** Fires once on mount, not on update — a remount is the only way this fires a second time. */
function MountSpy({ onMount }: { onMount: () => void }) {
  useEffect(() => { onMount(); }, [onMount]);
  return <main>Work surface</main>;
}

function region(overrides: Partial<HorizontalPaneRegion> & { content: React.ReactNode; label: string }): HorizontalPaneRegion {
  return {
    open: true,
    width: 280,
    min: 200,
    max: 480,
    onWidthChange: vi.fn(),
    onOpenChange: vi.fn(),
    ...overrides,
  };
}

const navigation = region({ content: <nav>Navigation content</nav>, label: "Navigation" });
const inspector = region({ content: <aside>Inspector content</aside>, label: "Inspector" });

describe("DestinationLayoutBody", () => {
  it("renders both navigation and inspector inline at the wide tier, not as sheets", () => {
    render(<DestinationLayoutBody containerWidth={1600} inspector={inspector} main={<main>Work surface</main>} navigation={navigation} tier="wide" />);
    expect(screen.getByText("Navigation content")).toBeTruthy();
    expect(screen.getByText("Inspector content")).toBeTruthy();
    expect(screen.getAllByRole("separator")).toHaveLength(2);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("collapses inspector before it can starve the work surface below MAIN_MIN_WIDTH at the wide tier", () => {
    render(
      <DestinationLayoutBody
        containerWidth={900}
        inspector={region({ ...inspector, width: 400 })}
        main={<main>Work surface</main>}
        navigation={region({ ...navigation, width: 400 })}
        tier="wide"
      />,
    );
    expect(screen.getByText("Navigation content")).toBeTruthy();
    expect(screen.queryByText("Inspector content")).toBeNull();
  });

  it("keeps navigation inline but demotes inspector to a sheet at the standard tier", () => {
    render(<DestinationLayoutBody containerWidth={1100} inspector={inspector} main={<main>Work surface</main>} navigation={navigation} tier="standard" />);
    expect(screen.getByText("Navigation content")).toBeTruthy();
    const dialog = screen.getByRole("dialog", { name: "Inspector" });
    expect(dialog).toBeTruthy();
  });

  it("demotes both to mutually exclusive sheets at the compact tier, with inspector taking priority", () => {
    render(<DestinationLayoutBody containerWidth={900} inspector={inspector} main={<main>Work surface</main>} navigation={navigation} tier="compact" />);
    expect(screen.getByRole("dialog", { name: "Inspector" })).toBeTruthy();
    expect(screen.queryByRole("dialog", { name: "Navigation" })).toBeNull();
  });

  it("shows navigation as a sheet at the compact tier once inspector is closed", () => {
    render(
      <DestinationLayoutBody
        containerWidth={900}
        inspector={region({ ...inspector, open: false })}
        main={<main>Work surface</main>}
        navigation={navigation}
        tier="compact"
      />,
    );
    expect(screen.getByRole("dialog", { name: "Navigation" })).toBeTruthy();
  });

  it("uses a full-viewport sheet placement at the narrow tier", () => {
    const { container } = render(
      <DestinationLayoutBody containerWidth={640} inspector={inspector} main={<main>Work surface</main>} navigation={navigation} tier="narrow" />,
    );
    expect(container.querySelector('[role="dialog"].inset-0')).not.toBeNull();
  });

  it("renders a vertical split for an open runtime panel, and plain content when closed", () => {
    const runtimePanel: RuntimePanelRegion = {
      content: <div>Runtime content</div>,
      open: true,
      height: 200,
      min: 120,
      max: 480,
      onHeightChange: vi.fn(),
      label: "Runtime panel",
    };
    const open = render(<DestinationLayoutBody containerWidth={1600} main={<main>Work surface</main>} runtimePanel={runtimePanel} tier="wide" />);
    expect(open.getByText("Runtime content")).toBeTruthy();
    open.unmount();

    render(<DestinationLayoutBody containerWidth={1600} main={<main>Work surface</main>} runtimePanel={{ ...runtimePanel, open: false }} tier="wide" />);
    expect(screen.queryByText("Runtime content")).toBeNull();
    expect(screen.getByText("Work surface")).toBeTruthy();
  });

  it("does not remount main when inspector or navigation toggles open at an inline tier", () => {
    const mounts = vi.fn();
    const renderWith = (inspectorOpen: boolean, navigationOpen: boolean) => (
      <DestinationLayoutBody
        containerWidth={1600}
        inspector={region({ ...inspector, open: inspectorOpen })}
        main={<MountSpy onMount={mounts} />}
        navigation={region({ ...navigation, open: navigationOpen })}
        tier="wide"
      />
    );
    const { rerender } = render(renderWith(true, true));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderWith(false, true));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderWith(false, false));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderWith(true, false));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderWith(true, true));
    expect(mounts).toHaveBeenCalledTimes(1);
  });

  it("does not remount main across a tier change, including a transient drop to narrow", () => {
    // A hidden container's ResizeObserver reports a momentary zero width before its real size
    // arrives (main-layout.tsx toggles the Sessions destination with CSS `hidden`, not unmount),
    // which classifies as `narrow` for one render — this reproduces that transient without needing
    // a real ResizeObserver.
    const mounts = vi.fn();
    const renderAt = (tier: "wide" | "narrow") => (
      <DestinationLayoutBody
        containerWidth={tier === "wide" ? 1600 : 0}
        inspector={inspector}
        main={<MountSpy onMount={mounts} />}
        navigation={navigation}
        tier={tier}
      />
    );
    const { rerender } = render(renderAt("wide"));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderAt("narrow"));
    expect(mounts).toHaveBeenCalledTimes(1);
    rerender(renderAt("wide"));
    expect(mounts).toHaveBeenCalledTimes(1);
  });

  it("tags its root with the current tier for downstream styling/testing hooks", () => {
    const { container } = render(<DestinationLayoutBody containerWidth={1600} main={<main>Work surface</main>} tier="wide" />);
    expect(container.querySelector('[data-layout-tier="wide"]')).not.toBeNull();
  });
});
