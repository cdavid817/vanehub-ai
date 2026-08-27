// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode, useRef, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  createLocalMediaDouble,
  readyStatus,
  transcription,
  type LocalMediaDouble,
} from "./local-media-test-double";
import { useLocalMediaComposer } from "./use-local-media-composer";

/**
 * Switching session while the microphone is live.
 *
 * The native side matches a recording by id *and* by the composer scope that started it, which is
 * what stops one window ending another's capture. That contract only holds if the frontend keeps
 * the scope the recording was born under; using whatever scope is on screen when the release
 * happens aims the call at the wrong session, the native side refuses it, and the
 * application-wide single-recording slot stays occupied with the microphone open.
 */
describe("switching session while the microphone is live", () => {
  let double: LocalMediaDouble;

  beforeEach(() => {
    double = createLocalMediaDouble(readyStatus());
    Element.prototype.setPointerCapture = vi.fn();
    Element.prototype.releasePointerCapture = vi.fn();
  });

  function Harness({ scope }: { scope: string | null }) {
    const [draft, setDraft] = useState("");
    const draftRef = useRef(draft);
    draftRef.current = draft;
    const model = useLocalMediaComposer({
      composerScopeId: scope,
      getDraft: () => draftRef.current,
      setDraft,
      getSelection: () => null,
      service: double.service,
    });
    return (
      <>
        <span data-testid="draft">{draft}</span>
        <span data-testid="phase">{model.microphonePhase}</span>
        <span data-testid="failure">{model.failure?.code ?? ""}</span>
        <button
          data-testid="mic"
          disabled={!model.availability.stt}
          type="button"
          {...model.microphone}
        >
          mic
        </button>
      </>
    );
  }

  // StrictMode throughout: the controller keeps mount-scoped refs, and its remount is the cheapest
  // way to catch one that a teardown clears without restoring.
  function mount(scope = "session-a") {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const tree = (next: string | null) => (
      <StrictMode>
        <QueryClientProvider client={client}>
          <Harness scope={next} />
        </QueryClientProvider>
      </StrictMode>
    );
    const view = render(tree(scope));
    return { view, switchTo: (next: string | null) => act(async () => view.rerender(tree(next))) };
  }

  const micButton = () => screen.getByTestId("mic") as HTMLButtonElement;
  const phase = () => screen.getByTestId("phase").textContent;
  const press = () =>
    act(() => {
      fireEvent.pointerDown(screen.getByTestId("mic"), { button: 0, pointerId: 1 });
    });
  const release = () =>
    act(() => {
      fireEvent.pointerUp(screen.getByTestId("mic"), { button: 0, pointerId: 1 });
    });
  const escape = () =>
    act(() => {
      fireEvent.keyDown(window, { key: "Escape" });
    });
  /**
   * Whether a hold actually starts.
   *
   * The button's `disabled` attribute reflects only whether the engine is configured -- the real
   * composer works the same way -- so "the microphone is usable again" can only be asked by
   * pressing it and seeing whether the native side was asked for a recording.
   */
  const holdStarts = async () => {
    const before = double.calls.startRecording.length;
    await press();
    await settle();
    return double.calls.startRecording.length > before;
  };

  const settle = () =>
    act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

  /** Mount, arm the control, and get session A into a running recording. */
  async function recordingInSessionA() {
    const mounted = mount();
    await waitFor(() => expect(micButton().disabled).toBe(false));
    await press();
    await settle();
    expect(phase()).toBe("recording");
    return mounted;
  }

  it("cancels the running recording as the session that started it", async () => {
    const mounted = await recordingInSessionA();

    await mounted.switchTo("session-b");
    await settle();

    // Not `rec-1@session-b`: that is the call the native side refuses, and refusing it is what
    // would leave the microphone open with the slot held.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    // Nothing the user did not ask for gets transcribed into a session they just arrived in.
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
  });

  it("returns the control to the session the user switched to", async () => {
    const mounted = await recordingInSessionA();

    await mounted.switchTo("session-b");
    await settle();

    expect(phase()).toBe("idle");
    expect(await holdStarts()).toBe(true);
  });

  it("lets the new session record once the old recording has been released", async () => {
    const mounted = await recordingInSessionA();
    await mounted.switchTo("session-b");
    await settle();

    await press();
    await settle();
    await release();
    await settle();

    expect(double.calls.startRecording).toEqual(["session-a", "session-b"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual(["rec-2@session-b"]);
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
  });

  it("keeps the new session's transcript after releasing the previous session", async () => {
    const mounted = await recordingInSessionA();
    await mounted.switchTo("session-b");
    await settle();

    await press();
    await settle();
    await release();
    await settle();
    double.settle("op-1", { kind: "stt", result: transcription("words from session b") });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 400));
    });

    expect(screen.getByTestId("draft").textContent).toBe("words from session b");
  });

  it("keeps the abandoned recording's transcript out of the new session's draft", async () => {
    const mounted = await recordingInSessionA();
    await mounted.switchTo("session-b");
    await settle();
    // Even if a result for the abandoned operation were published, nothing is watching it.
    double.settle("op-1", { kind: "stt", result: transcription("words from session a") });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 400));
    });

    expect(screen.getByTestId("draft").textContent).toBe("");
    expect(screen.getByTestId("failure").textContent).toBe("");
  });

  it("ignores a release that arrives after the session changed", async () => {
    const mounted = await recordingInSessionA();
    await mounted.switchTo("session-b");
    await settle();

    await release();
    await settle();

    // The recording is already gone; a release cannot resurrect it into the new session.
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(phase()).toBe("idle");
  });

  it("ignores an Escape that arrives after the session changed", async () => {
    const mounted = await recordingInSessionA();
    await mounted.switchTo("session-b");
    await settle();

    await escape();
    await settle();

    // Exactly one release, not a second one aimed at a recording that no longer exists.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(phase()).toBe("idle");
  });

  it("releases once across two rapid session changes", async () => {
    const mounted = await recordingInSessionA();

    await mounted.switchTo("session-b");
    await mounted.switchTo("session-c");
    await settle();

    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(phase()).toBe("idle");
    expect(await holdStarts()).toBe(true);
  });

  it("does not release anything when the composer first mounts", async () => {
    mount();
    await waitFor(() => expect(micButton().disabled).toBe(false));
    await settle();

    // StrictMode mounts, tears down, and remounts. None of that is a session change.
    expect(double.calls.cancelRecording).toEqual([]);
    expect(double.calls.startRecording).toEqual([]);
  });

  it("refuses to re-arm while a failed release leaves the microphone unaccounted for", async () => {
    const mounted = await recordingInSessionA();
    double.failNextCancelRecording("TEMP_STORAGE_FAILED");

    await mounted.switchTo("session-b");
    await settle();

    // The native recording may still be open. Accepting a new hold would claim the microphone is
    // closed on no evidence, and the second one would collide with the first on the
    // application-wide slot.
    expect(await holdStarts()).toBe(false);
    expect(screen.getByTestId("failure").textContent).toBe("TEMP_STORAGE_FAILED");
  });

  it("re-arms when the release failed only because there was nothing left to release", async () => {
    const mounted = await recordingInSessionA();
    double.failNextCancelRecording("RECORDING_NOT_FOUND");

    await mounted.switchTo("session-b");
    await settle();

    // That code is the end state this was trying to reach: the native side holds no such
    // recording, so the slot is free and the new session may use it.
    expect(await holdStarts()).toBe(true);
    expect(screen.getByTestId("failure").textContent).toBe("");
  });

  it("does not update state after the composer unmounts mid-recording", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const mounted = await recordingInSessionA();

    await act(async () => mounted.view.unmount());
    await settle();

    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(errors).not.toHaveBeenCalled();
    errors.mockRestore();
  });
});
