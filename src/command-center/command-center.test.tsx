/** @vitest-environment jsdom */

import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import { activateAppLanguage } from "../i18n";
import { CommandCenter } from "./command-center";
import { commandCenterShortcutLabel } from "./platform";
import type { WorkbenchCommandContext, WorkbenchSearchPage, WorkbenchSearchProvider, WorkbenchSearchResult } from "./command-center-types";

// A leaked fake-timer state from a failed assertion mid-test would hang every subsequent test's
// own real-timer debounce waits — a global safety net, same rationale as the orchestration hook's
// own test file.
afterEach(() => vi.useRealTimers());

function makeContext(overrides: Partial<WorkbenchCommandContext> = {}): WorkbenchCommandContext {
  return {
    location: { destination: "sessions", sessionId: null, creatingSession: false },
    navigate: vi.fn(),
    onOpenSettings: vi.fn(),
    onNewSession: vi.fn(),
    onToggleNavigation: vi.fn(),
    onToggleInspector: vi.fn(),
    onToggleFocusMode: vi.fn(),
    ...overrides,
  };
}

function fakeResult(overrides: Partial<WorkbenchSearchResult> = {}): WorkbenchSearchResult {
  return {
    key: "r1",
    kind: "session",
    title: "auth session",
    route: { destination: "sessions", sessionId: "s-1", creatingSession: false },
    ...overrides,
  };
}

function fakeProvider(page: () => Promise<WorkbenchSearchPage>): WorkbenchSearchProvider {
  return { id: "fake", supports: () => true, search: () => page() };
}

// Bare `i18n.changeLanguage("en")` sets `i18n.language` without loading English's resource
// bundle — only `defaultAppLanguage` (zh-CN) is bundled eagerly (i18n/index.ts); every other
// locale, including English, is behind a dynamic `import()` that a raw `changeLanguage` call never
// triggers, so `t()` would silently keep serving the zh-CN fallback. `activateAppLanguage` is the
// same function the real language switcher calls — it loads the bundle first.
async function open(overrides: Partial<WorkbenchCommandContext> = {}, providers: WorkbenchSearchProvider[] = []) {
  await activateAppLanguage("en");
  const context = makeContext(overrides);
  const onClose = vi.fn();
  const rendered = renderWithAppProviders(<CommandCenter context={context} onClose={onClose} providers={providers} />);
  return { ...rendered, context, onClose };
}

describe("CommandCenter", () => {
  it("names itself and its shortcut hint for a screen reader, in the current locale", async () => {
    await open();

    expect(screen.getByRole("dialog", { name: "Command Center" })).toBeTruthy();
    expect(screen.getByRole("combobox")).toBeTruthy();
    // Calls the real platform detector rather than asserting a fixed "Ctrl+K": a Mac CI runner
    // would make a hardcoded expectation wrong without this actually being a regression.
    expect(screen.getByText(`Press ${commandCenterShortcutLabel()} to open`)).toBeTruthy();
  });

  it("lists every available command when the query is empty", async () => {
    await open();
    expect(screen.getByRole("option", { name: "Go to Projects" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "New Session" })).toBeTruthy();
  });

  it("hides destination-scoped contextual commands outside Sessions, per 6.9", async () => {
    await open({ location: { destination: "projects", projectId: undefined } });
    expect(screen.queryByRole("option", { name: "Toggle Focus Mode" })).toBeNull();
    expect(screen.queryByRole("option", { name: "Toggle Info Panel" })).toBeNull();
    // Destination commands and the always-available New Session are unaffected.
    expect(screen.getByRole("option", { name: "Go to Runs" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "New Session" })).toBeTruthy();
  });

  it("filters commands by keyword or label as the query changes", async () => {
    await open();
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "focus" } });
    expect(screen.getByRole("option", { name: "Toggle Focus Mode" })).toBeTruthy();
    expect(screen.queryByRole("option", { name: "New Session" })).toBeNull();
  });

  it("shows a matching search result alongside matching commands", async () => {
    // "auth" does not appear in any command's own label or keywords, so this proves the row came
    // from the provider, not from a coincidental command match.
    await open({}, [fakeProvider(() => Promise.resolve({ items: [fakeResult({ title: "auth token" })], nextCursor: null }))]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "auth" } });

    await waitFor(() => expect(screen.getByRole("option", { name: "auth token" })).toBeTruthy());
    expect(screen.queryAllByRole("option")).toHaveLength(1);
  });

  it("selecting a result with Enter navigates to its route and closes", async () => {
    const { context, onClose } = await open({}, [
      fakeProvider(() => Promise.resolve({ items: [fakeResult({ title: "auth token" })], nextCursor: null })),
    ]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "auth" } });
    await waitFor(() => expect(screen.getByRole("option", { name: "auth token" })).toBeTruthy());

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });

    expect(context.navigate).toHaveBeenCalledWith(fakeResult().route);
    expect(onClose).toHaveBeenCalled();
  });

  it("moves past a result onto a command with the arrow keys and runs it with Enter", async () => {
    // "settings" uniquely matches exactly one command (open-settings) and the fixture result below,
    // so the second row is unambiguously the command.
    const { context, onClose } = await open({}, [
      fakeProvider(() => Promise.resolve({ items: [fakeResult({ title: "settings backup" })], nextCursor: null })),
    ]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "settings" } });
    await waitFor(() => expect(screen.getByRole("option", { name: "settings backup" })).toBeTruthy());
    expect(screen.getByRole("option", { name: "Open Settings" })).toBeTruthy();

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });

    expect(context.onOpenSettings).toHaveBeenCalled();
    expect(context.navigate).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("resets the highlight when an async result lands after a command was already highlighted", async () => {
    // "session" matches three commands synchronously (Go to Sessions, New Session, Toggle Session
    // List) before the provider below ever resolves.
    let resolveSearch: ((page: WorkbenchSearchPage) => void) | undefined;
    const { context } = await open({}, [fakeProvider(() => new Promise((resolve) => { resolveSearch = resolve; }))]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "session" } });
    await waitFor(() => expect(screen.getByRole("option", { name: "New Session" })).toBeTruthy());

    // Highlights the second command (New Session) before the result arrives.
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "ArrowDown" });

    // Commands filter synchronously, but the debounced search settles later — waiting for "New
    // Session" above proves nothing about whether `resolveSearch` has been assigned yet. Wait for
    // the loading indicator (only true once the debounce fires and `provider.search` is actually
    // called) before resolving it, or this resolves a promise that does not exist yet.
    await waitFor(() => expect(screen.getByText("Searching...")).toBeTruthy());
    resolveSearch?.({ items: [fakeResult({ title: "session helper" })], nextCursor: null });
    // The new option appearing only proves `search.results` itself has landed -- the reset effect
    // (`useEffect(() => setActive(0), [query, search.results])`) is a separate, effect-driven
    // update that can still be pending in the same instant this first assertion passes, since
    // React flushes passive effects after the commit that satisfies it. Waiting for this option's
    // own `aria-selected` too, not just its presence, proves the reset has actually settled before
    // the Enter below fires against it -- without this the highlighted index can still be the
    // pre-reset one for a moment, aiming Enter at the wrong entry.
    await waitFor(() => expect(screen.getByRole("option", { name: "session helper" }).getAttribute("aria-selected")).toBe("true"));

    // The result's arrival inserted a new row ahead of every command, shifting New Session down a
    // slot. Without the reset, the still-highlighted index would now land on Go to Sessions instead
    // — a materially different action from the one the reader was looking at.
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
    expect(context.navigate).toHaveBeenCalledWith(fakeResult().route);
  });

  it("does not walk past either end of the combined list", async () => {
    const { context } = await open();
    const input = screen.getByRole("combobox");
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });

    // Clamped at the first entry (a destination command, since the query is empty and no provider
    // is configured) rather than wrapping to the last.
    expect(context.navigate).toHaveBeenCalledWith({ destination: "sessions", sessionId: null, creatingSession: false });
  });

  it("closes on Escape", async () => {
    const { onClose } = await open();
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("shows a loading indicator while a search is in flight, then not once it resolves", async () => {
    let resolveSearch: ((page: WorkbenchSearchPage) => void) | undefined;
    await open({}, [fakeProvider(() => new Promise((resolve) => { resolveSearch = resolve; }))]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "auth" } });

    await waitFor(() => expect(screen.getByText("Searching...")).toBeTruthy());
    resolveSearch?.({ items: [fakeResult({ title: "auth token" })], nextCursor: null });
    await waitFor(() => expect(screen.queryByText("Searching...")).toBeNull());
    expect(screen.getByRole("option", { name: "auth token" })).toBeTruthy();
  });

  it("shows a partial-failure notice when a provider rejects without losing the others' results", async () => {
    await open({}, [
      fakeProvider(() => Promise.resolve({ items: [fakeResult({ title: "auth token" })], nextCursor: null })),
      { id: "broken", supports: () => true, search: () => Promise.reject(new Error("network error")) },
    ]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "auth" } });

    await waitFor(() => expect(screen.getByRole("option", { name: "auth token" })).toBeTruthy());
    expect(screen.getByText("Some results may be incomplete.")).toBeTruthy();
  });

  it("never renders a result's status, updatedAt, or keywords as visible text", async () => {
    await open({}, [fakeProvider(() => Promise.resolve({
      items: [fakeResult({ title: "auth token", status: "error", updatedAt: "2026-01-01T00:00:00Z", keywords: ["do-not-leak-this-keyword"] })],
      nextCursor: null,
    }))]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "auth" } });

    await waitFor(() => expect(screen.getByRole("option", { name: "auth token" })).toBeTruthy());
    expect(screen.queryByText("do-not-leak-this-keyword")).toBeNull();
    expect(screen.queryByText("error")).toBeNull();
    expect(screen.queryByText(/2026-01-01/)).toBeNull();
  });
});
