// @vitest-environment jsdom

import { useEffect } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import type { LazyFeatureLoader } from "../components/lazy-feature";
import { renderWithAppProviders } from "../test/render";
import type { SettingsDraftGuard, SettingsPageContext } from "./settings-page-types";
import { SettingsShell } from "./settings-shell";

function renderShell(onReturn?: () => void) {
  return renderWithAppProviders(
    <MemoryRouter>
      <SettingsShell onReturn={onReturn} />
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

/** Reports a fixed guard on mount (or none, with `null`) via `onDraftStateChange` — a stand-in for
 *  a real page's own dirty-draft tracking, so the shell's navigation guard can be exercised
 *  without depending on any specific real page's draft implementation. */
function draftyMockPage(label: string, guard: SettingsDraftGuard | null) {
  function DraftyMockPage({ onDraftStateChange }: { onDraftStateChange?: (next: SettingsDraftGuard | null) => void }) {
    useEffect(() => {
      onDraftStateChange?.(guard);
      return () => onDraftStateChange?.(null);
    }, [onDraftStateChange]);
    return <p>{label}</p>;
  }
  return () => Promise.resolve({ default: DraftyMockPage });
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

describe("SettingsShell draft navigation guard (task 12.12)", () => {
  it("does not intercept an inter-page switch away from a keepAlive: draft-only page's dirty draft", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const discard = vi.fn();
    mockLoaders.never.mockImplementation(mountSpyPage("Basic page", vi.fn()));
    mockLoaders.draftOnly.mockImplementation(draftyMockPage("CLI parameters page", { canSave: true, discard, dirtyCount: 1, save }));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "基础配置" }));
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    // No prompt at all: draft-only survives the switch on its own (task 12.17), so nothing needed
    // saving/discarding/confirming just to look at a different page.
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(save).not.toHaveBeenCalled();
    expect(discard).not.toHaveBeenCalled();
  });

  it("intercepts an inter-page switch away from a keepAlive: never page's dirty draft, and Stay cancels it", async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const discard = vi.fn();
    mockLoaders.never.mockImplementation(draftyMockPage("Basic page", { canSave: true, discard, dirtyCount: 3, save }));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", vi.fn()));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("3");

    fireEvent.click(screen.getByRole("button", { name: "留在此页" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Still on Basic: the switch never happened.
    expect(screen.getByText("Basic page")).toBeTruthy();
    expect(screen.queryByText("CLI parameters page")).toBeNull();
    expect(save).not.toHaveBeenCalled();
    expect(discard).not.toHaveBeenCalled();
  });

  it("Discard proceeds with the switch after clearing the reported draft", async () => {
    const discard = vi.fn();
    mockLoaders.never.mockImplementation(draftyMockPage("Basic page", { canSave: true, discard, dirtyCount: 1, save: vi.fn() }));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", vi.fn()));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: "放弃更改" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());
    expect(discard).toHaveBeenCalledOnce();
  });

  it("Save awaits the reported save before proceeding with the switch", async () => {
    let resolveSave: () => void = () => {};
    const save = vi.fn(() => new Promise<void>((resolve) => { resolveSave = resolve; }));
    mockLoaders.never.mockImplementation(draftyMockPage("Basic page", { canSave: true, discard: vi.fn(), dirtyCount: 1, save }));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", vi.fn()));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: "保存并离开" }));
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    // Still on Basic, mid-save: the switch waits for the promise, it does not race ahead of it.
    expect(screen.getByText("Basic page")).toBeTruthy();

    resolveSave();
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());
  });

  it("keeps the user on the page instead of leaving when Save rejects", async () => {
    const save = vi.fn().mockRejectedValue(new Error("boom"));
    mockLoaders.never.mockImplementation(draftyMockPage("Basic page", { canSave: true, discard: vi.fn(), dirtyCount: 1, save }));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", vi.fn()));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: "保存并离开" }));
    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // The failed save did not navigate away as though it had succeeded.
    expect(screen.getByText("Basic page")).toBeTruthy();
    expect(screen.queryByText("CLI parameters page")).toBeNull();
  });

  it("guards leaving Settings entirely regardless of the active page's own keepAlive policy", async () => {
    const onReturn = vi.fn();
    const discard = vi.fn();
    // draft-only: an inter-page switch would not need this, but leaving Settings unmounts the
    // whole shell, which no per-page keepAlive policy protects against.
    mockLoaders.never.mockImplementation(mountSpyPage("Basic page", vi.fn()));
    mockLoaders.draftOnly.mockImplementation(draftyMockPage("CLI parameters page", { canSave: true, discard, dirtyCount: 1, save: vi.fn() }));

    renderShell(onReturn);
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    await waitFor(() => expect(screen.getByText("CLI parameters page")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "返回" }));
    await screen.findByRole("dialog");
    expect(onReturn).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "放弃更改" }));
    await waitFor(() => expect(onReturn).toHaveBeenCalledOnce());
    expect(discard).toHaveBeenCalledOnce();
  });

  it("never exposes a reported draft's own data through the shell, only a count and opaque callbacks", async () => {
    // The type itself only carries a count and two callbacks -- there is no field for a value to
    // occupy. This asserts the one place that could still leak one by accident: the rendered
    // prompt's own text, which only ever interpolates `dirtyCount`.
    const secretValue = "sk-do-not-leak-this-token";
    mockLoaders.never.mockImplementation(draftyMockPage("Basic page", {
      canSave: true,
      discard: () => { void secretValue; },
      dirtyCount: 1,
      save: async () => { void secretValue; },
    }));
    mockLoaders.draftOnly.mockImplementation(mountSpyPage("CLI parameters page", vi.fn()));

    renderShell();
    await waitFor(() => expect(screen.getByText("Basic page")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "CLI 参数" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).not.toContain(secretValue);
    expect(document.body.textContent).not.toContain(secretValue);
  });
});
