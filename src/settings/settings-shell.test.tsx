// @vitest-environment jsdom

import { useEffect } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import type { LazyFeatureLoader } from "../components/lazy-feature";
import { renderWithAppProviders } from "../test/render";
import type { SettingsPageContext } from "./settings-page-types";
import { SettingsShell } from "./settings-shell";

function renderShell() {
  return renderWithAppProviders(
    <MemoryRouter>
      <SettingsShell />
    </MemoryRouter>,
  );
}

/** Fires once on mount, not on update — a remount is the only way this fires a second time. */
function mountSpyPage(label: string, onMount: () => void) {
  function MountSpyPage() {
    useEffect(() => { onMount(); }, []);
    return <p>{label}</p>;
  }
  return () => Promise.resolve({ default: MountSpyPage });
}

// vi.mock factories run once, at import time, before any it() block — so the mocked loaders can't
// close over per-test spies directly. Route through vi.hoisted() instead: it gives the factory a
// stable indirection target whose *implementation* each it() swaps in later, at call time.
const mockLoaders = vi.hoisted(() => ({
  never: vi.fn<LazyFeatureLoader<SettingsPageContext>>(() => Promise.resolve({ default: () => null })),
  draftOnly: vi.fn<LazyFeatureLoader<SettingsPageContext>>(() => Promise.resolve({ default: () => null })),
}));

vi.mock("./settings-pages", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./settings-pages")>();
  return {
    ...actual,
    defaultSettingsPageId: "basic",
    settingsPages: [
      { ...actual.settingsPages.find((page) => page.id === "basic")!, loader: () => mockLoaders.never() },
      { ...actual.settingsPages.find((page) => page.id === "cli-parameters")!, loader: () => mockLoaders.draftOnly() },
    ],
    getSettingsPage: (id: string) => actual.settingsPages.find((page) => page.id === id) ?? actual.settingsPages[0],
  };
});

describe("SettingsShell page lifecycle", () => {
  it("unmounts a keepAlive:never page when navigating away, and remounts it on return", async () => {
    const neverMounts = vi.fn();
    const draftOnlyMounts = vi.fn();
    mockLoaders.never.mockImplementation(mountSpyPage("Basic page", neverMounts));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", draftOnlyMounts));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    expect(neverMounts).toHaveBeenCalledTimes(1);
    expect(draftOnlyMounts).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());
    // The never-policy page it left is gone from the DOM entirely, not just hidden.
    expect(screen.queryByText("Basic page")).toBeNull();
    expect(draftOnlyMounts).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "基础配置" }));
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    // Remounted: a fresh instance, not the one from the first visit.
    expect(neverMounts).toHaveBeenCalledTimes(2);
  });

  it("keeps a keepAlive:draft-only page mounted (hidden, not removed) once visited", async () => {
    const neverMounts = vi.fn();
    const draftOnlyMounts = vi.fn();
    mockLoaders.never.mockImplementation(mountSpyPage("Basic page", neverMounts));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", draftOnlyMounts));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());
    expect(draftOnlyMounts).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "基础配置" }));
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());
    // Still the same instance: staying mounted, hidden rather than unmounted, is the point of
    // draft-only — it never fires its mount effect a second time.
    expect(draftOnlyMounts).toHaveBeenCalledTimes(1);
  });
});
