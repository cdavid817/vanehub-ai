// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useRunConfigurationOverrides } from "./useRunConfigurationOverrides";
import type { ChatConfig } from "../../../types/chat";

function profile(overrides: Partial<ChatConfig> = {}): ChatConfig {
  return {
    agentId: "claude-code",
    interactionMode: "cli",
    executionMode: "inherit",
    providerId: "anthropic",
    modelId: "claude-sonnet-4-6",
    reasoningDepth: "high",
    streaming: true,
    thinking: true,
    longContext: false,
    ...overrides,
  };
}

describe("useRunConfigurationOverrides", () => {
  it("starts with every field sourced from the profile, no overrides active", () => {
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", profile()));
    expect(result.current.hasOverrides).toBe(false);
    expect(result.current.sourceOf("modelId")).toBe("profile");
    expect(result.current.effectiveConfig).toEqual(profile());
  });

  it("reflects a staged override in the effective config without touching the profile value passed in", () => {
    const baseProfile = profile();
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", baseProfile));
    act(() => result.current.setOverride("modelId", "claude-opus-5"));
    expect(result.current.effectiveConfig.modelId).toBe("claude-opus-5");
    expect(result.current.sourceOf("modelId")).toBe("override");
    expect(baseProfile.modelId).toBe("claude-sonnet-4-6");
  });

  it("leaves every other field sourced from the profile when only one field is overridden", () => {
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", profile()));
    act(() => result.current.setOverride("thinking", false));
    expect(result.current.sourceOf("thinking")).toBe("override");
    expect(result.current.sourceOf("modelId")).toBe("profile");
    expect(result.current.effectiveConfig.modelId).toBe(profile().modelId);
  });

  it("reverts a single field back to the profile value with resetOverride", () => {
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", profile()));
    act(() => result.current.setOverride("modelId", "claude-opus-5"));
    act(() => result.current.resetOverride("modelId"));
    expect(result.current.sourceOf("modelId")).toBe("profile");
    expect(result.current.effectiveConfig.modelId).toBe(profile().modelId);
    expect(result.current.hasOverrides).toBe(false);
  });

  it("reverts every field at once with resetAllOverrides", () => {
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", profile()));
    act(() => {
      result.current.setOverride("modelId", "claude-opus-5");
      result.current.setOverride("thinking", false);
    });
    act(() => result.current.resetAllOverrides());
    expect(result.current.hasOverrides).toBe(false);
    expect(result.current.effectiveConfig).toEqual(profile());
  });

  it("clears every override after a send goes through", () => {
    const { result } = renderHook(() => useRunConfigurationOverrides("session-1", profile()));
    act(() => result.current.setOverride("modelId", "claude-opus-5"));
    act(() => result.current.clearAfterSend());
    expect(result.current.hasOverrides).toBe(false);
  });

  it("clears stale overrides when the session changes, rather than leaking them into a different session", () => {
    const { rerender, result } = renderHook(
      ({ sessionId }) => useRunConfigurationOverrides(sessionId, profile()),
      { initialProps: { sessionId: "session-1" } },
    );
    act(() => result.current.setOverride("modelId", "claude-opus-5"));
    expect(result.current.hasOverrides).toBe(true);
    rerender({ sessionId: "session-2" });
    expect(result.current.hasOverrides).toBe(false);
    expect(result.current.effectiveConfig.modelId).toBe(profile().modelId);
  });

  it("keeps an override active even after the underlying profile value it is masking changes", () => {
    // Overrides are keyed by field, not by value, and only clear on an explicit reset, a send, or
    // a session change (all tested above) -- a same-session profile edit underneath an active
    // override (e.g. a background sync, or the reader also touching "change my default" for an
    // unrelated field that happens to recompute this one) must not silently un-mask it.
    const { rerender, result } = renderHook(
      ({ config }) => useRunConfigurationOverrides("session-1", config),
      { initialProps: { config: profile() } },
    );
    act(() => result.current.setOverride("modelId", "claude-opus-5"));
    rerender({ config: profile({ modelId: "claude-haiku-4-5" }) });
    expect(result.current.effectiveConfig.modelId).toBe("claude-opus-5");
    expect(result.current.sourceOf("modelId")).toBe("override");
  });
});
