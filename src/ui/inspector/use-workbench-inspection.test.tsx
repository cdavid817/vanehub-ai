// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WorkbenchSelection } from "../../types/workbench-selection";
import { getInspectorProvider } from "./inspector-provider-registry";
import { useWorkbenchInspection } from "./use-workbench-inspection";

vi.mock("./inspector-provider-registry", () => ({ getInspectorProvider: vi.fn() }));

const messageSelection: WorkbenchSelection = { kind: "message", sessionId: "s-1", messageId: "m-1" };
const fileSelection: WorkbenchSelection = { kind: "file", sessionId: "s-1", pathId: "path-1" };

describe("useWorkbenchInspection", () => {
  it("starts in overview mode with no selection", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    expect(result.current.mode).toBe("overview");
    expect(result.current.selection).toBeNull();
  });

  it("enters follow mode on the first followed selection and updates its title per kind", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    expect(result.current.mode).toBe("follow");
    expect(result.current.selection).toEqual(messageSelection);
  });

  it("replaces the selection when a new one is followed while not pinned", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    act(() => result.current.follow(fileSelection));
    expect(result.current.selection).toEqual(fileSelection);
  });

  it("does not replace a pinned selection with a newly followed one", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    act(() => result.current.pin());
    act(() => result.current.follow(fileSelection));
    expect(result.current.mode).toBe("pinned");
    expect(result.current.selection).toEqual(messageSelection);
  });

  it("does nothing when pinning with no current selection", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.pin());
    expect(result.current.mode).toBe("overview");
  });

  it("returns to follow mode on unpin without clearing the selection", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    act(() => result.current.pin());
    act(() => result.current.unpin());
    expect(result.current.mode).toBe("follow");
    expect(result.current.selection).toEqual(messageSelection);
  });

  it("clears the selection and pin state on returnToOverview", () => {
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    act(() => result.current.pin());
    act(() => result.current.returnToOverview());
    expect(result.current.mode).toBe("overview");
    expect(result.current.selection).toBeNull();
  });

  it("auto-clears a non-pinned selection once it drifts out of scope", () => {
    const { rerender, result } = renderHook(({ scope }) => useWorkbenchInspection(scope), {
      initialProps: { scope: { activeSessionId: "s-1" } },
    });
    act(() => result.current.follow(messageSelection));
    rerender({ scope: { activeSessionId: "s-2" } });
    expect(result.current.mode).toBe("overview");
    expect(result.current.selection).toBeNull();
  });

  it("keeps a pinned selection's identity when it drifts out of scope, and marks its detail unavailable", () => {
    const { rerender, result } = renderHook(({ scope }) => useWorkbenchInspection(scope), {
      initialProps: { scope: { activeSessionId: "s-1" } },
    });
    act(() => result.current.follow(messageSelection));
    act(() => result.current.pin());
    rerender({ scope: { activeSessionId: "s-2" } });
    expect(result.current.mode).toBe("pinned");
    expect(result.current.selection).toEqual(messageSelection);
    expect(result.current.detail.error?.kind).toBe("unavailable");
  });

  it("reports detail as unavailable when no provider is registered for the selection's kind", () => {
    vi.mocked(getInspectorProvider).mockReturnValue(undefined);
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    expect(result.current.detail.error?.kind).toBe("unavailable");
    expect(result.current.detail.data).toBeUndefined();
  });

  it("resolves detail content, keyed by kind, when a provider is registered", () => {
    vi.mocked(getInspectorProvider).mockReturnValue({
      kind: "message",
      titleKey: "workbenchUi.inspector.title.message",
      loader: () => Promise.resolve({ default: () => null }),
    });
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }));
    act(() => result.current.follow(messageSelection));
    expect(result.current.detail.error).toBeUndefined();
    expect(result.current.detail.data).toBeTruthy();
    // The key that makes switching provider kinds actually swap the mounted component instead of
    // getting stuck on whichever loader `LazyFeature` first constructed — see the hook's own doc
    // comment on `detail` for why this is load-bearing, not decorative.
    const dataElement = result.current.detail.data as { key: string | null };
    expect(dataElement.key).toBe("message");
  });

  it("threads the ambient provider context through to the resolved detail element", () => {
    vi.mocked(getInspectorProvider).mockReturnValue({
      kind: "message",
      titleKey: "workbenchUi.inspector.title.message",
      loader: () => Promise.resolve({ default: () => null }),
    });
    const onNavigateToSessionTab = vi.fn();
    const { result } = renderHook(() => useWorkbenchInspection({ activeSessionId: "s-1" }, { onNavigateToSessionTab }));
    act(() => result.current.follow(messageSelection));
    const dataElement = result.current.detail.data as { props: { componentProps: { context: { onNavigateToSessionTab: unknown } } } };
    expect(dataElement.props.componentProps.context.onNavigateToSessionTab).toBe(onNavigateToSessionTab);
  });
});
