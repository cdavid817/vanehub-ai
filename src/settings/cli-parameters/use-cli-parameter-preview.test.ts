// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useCliParameterPreview } from "./use-cli-parameter-preview";
import type { CliParameterSelections } from "../../types/cli-parameter";
import type { CliParameterPreview } from "../../types/cli-parameter-profile";

const previewCliParameterProfile = vi.fn();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    previewCliParameterProfile: (input: unknown) => previewCliParameterProfile(input),
  },
}));

function reply(token: string): CliParameterPreview {
  return {
    agentId: "claude-code",
    catalogVersion: "2.0.0",
    scope: "chat",
    normalizedSelections: {},
    segments: {
      global: [{ value: token, parameterId: "model", segment: "global" }],
      invocation: [],
    },
    diagnostics: [],
  };
}

const inherited: CliParameterSelections = { model: { state: "inherit" } };

describe("CLI parameter preview controller", () => {
  beforeEach(() => {
    previewCliParameterProfile.mockReset();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("debounces so a burst of keystrokes issues one request", async () => {
    previewCliParameterProfile.mockResolvedValue(reply("first"));
    const { rerender } = renderHook(
      ({ selections }: { selections: CliParameterSelections }) =>
        useCliParameterPreview("claude-code", "2.0.0", "chat", selections, true),
      { initialProps: { selections: inherited } },
    );

    rerender({ selections: { model: { state: "value", value: "o" } } });
    rerender({ selections: { model: { state: "value", value: "op" } } });
    rerender({ selections: { model: { state: "value", value: "opu" } } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });

    expect(previewCliParameterProfile).toHaveBeenCalledTimes(1);
    expect(previewCliParameterProfile.mock.calls[0][0].selections).toEqual({
      model: { state: "value", value: "opu" },
    });
  });

  it("ignores a slow response for an abandoned draft", async () => {
    let resolveFirst: ((value: CliParameterPreview) => void) | undefined;
    previewCliParameterProfile
      .mockImplementationOnce(
        () =>
          new Promise<CliParameterPreview>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(reply("second"));

    const { result, rerender } = renderHook(
      ({ selections }: { selections: CliParameterSelections }) =>
        useCliParameterPreview("claude-code", "2.0.0", "chat", selections, true),
      { initialProps: { selections: { model: { state: "value", value: "a" } } } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });

    rerender({ selections: { model: { state: "value", value: "b" } } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    expect(result.current.preview?.segments.global[0].value).toBe("second");

    // The first request lands last, and is discarded because its identity is no longer current.
    await act(async () => {
      resolveFirst?.(reply("first"));
      await vi.advanceTimersByTimeAsync(10);
    });
    expect(result.current.preview?.segments.global[0].value).toBe("second");
  });

  it("keeps the last valid preview when a later draft is rejected", async () => {
    previewCliParameterProfile
      .mockResolvedValueOnce(reply("valid"))
      .mockRejectedValueOnce({ code: "CLI_PARAMETER_INVALID_VALUE", parameterId: "model" });

    const { result, rerender } = renderHook(
      ({ selections }: { selections: CliParameterSelections }) =>
        useCliParameterPreview("claude-code", "2.0.0", "chat", selections, true),
      { initialProps: { selections: { model: { state: "value", value: "ok" } } } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    expect(result.current.preview?.segments.global[0].value).toBe("valid");

    rerender({ selections: { model: { state: "value", value: "!!" } } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });

    expect(result.current.error).not.toBeNull();
    expect(result.current.preview?.segments.global[0].value).toBe("valid");
    expect(result.current.stale).toBe(true);
  });

  it("issues nothing while disabled", async () => {
    renderHook(() => useCliParameterPreview("claude-code", "2.0.0", "chat", inherited, false));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });

    expect(previewCliParameterProfile).not.toHaveBeenCalled();
  });
});
