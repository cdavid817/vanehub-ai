// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode, useRef, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { MicrophonePhase } from "./local-media-composer-types";
import {
  createLocalMediaDouble,
  readyStatus,
  transcription,
  type LocalMediaDouble,
} from "./local-media-test-double";
import { useLocalMediaComposer } from "./use-local-media-composer";

/**
 * The window between pressing the microphone and the native side answering.
 *
 * `startRecording` opens a device; on a real machine that is hundreds of milliseconds. Every other
 * test in this suite closes the window -- either by resolving the double immediately or by
 * awaiting the press before releasing -- so the instructions a user can give inside it were never
 * exercised. Each case here defers the answer and gives one.
 */
describe("the microphone opening window", () => {
  let double: LocalMediaDouble;
  let phases: MicrophonePhase[];

  beforeEach(() => {
    double = createLocalMediaDouble(readyStatus());
    phases = [];
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
    phases.push(model.microphonePhase);
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

  /**
   * Mounted inside `StrictMode` on purpose.
   *
   * The controller keeps mount-scoped refs, and StrictMode's mount/unmount/remount is the cheapest
   * way to catch one that is cleared on teardown but never restored -- which is exactly how the
   * first version of this fix cancelled every recording it started, but only in development.
   */
  function mount(scope: string | null = "session-a") {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const tree = (next: string | null) => (
      <StrictMode>
        <QueryClientProvider client={client}>
          <Harness scope={next} />
        </QueryClientProvider>
      </StrictMode>
    );
    const view = render(tree(scope));
    return { view, rerenderScope: (next: string | null) => view.rerender(tree(next)) };
  }

  // `toBeDisabled` is a jest-dom matcher and this repository does not install them.
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
  const blur = () =>
    act(() => {
      fireEvent.blur(window);
      window.dispatchEvent(new Event("blur"));
    });
  const answer = async () => {
    await act(async () => {
      double.resolveStartRecording();
      await Promise.resolve();
    });
  };

  async function armed(scope: string | null = "session-a") {
    const mounted = mount(scope);
    await waitFor(() => expect(micButton().disabled).toBe(false));
    double.deferStartRecording();
    return mounted;
  }

  it("transcribes a tap released before the device finished opening", async () => {
    await armed();
    await press();
    expect(phase()).toBe("opening");

    await release();
    await answer();

    // The release is honoured against the handle that arrived after it, rather than dropped.
    expect(double.calls.stopRecordingAndTranscribe).toEqual(["rec-1@session-a"]);
    expect(double.calls.cancelRecording).toEqual([]);
    // The control never sat in `recording` with nobody holding it, which is the state the old
    // code reached and stayed in.
    expect(phases).not.toContain("recording");
    expect(phase()).toBe("transcribing");
  });

  it("appends the tap's transcript to the draft and sends nothing", async () => {
    await armed();
    await press();
    await release();
    await answer();
    double.settle("op-1", { kind: "stt", result: transcription("tapped words") });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 400));
    });

    expect(screen.getByTestId("draft").textContent).toBe("tapped words");
    // Nothing in this path submits: the transcript is a draft edit.
    expect(double.calls.startTts).toEqual([]);
    expect(phase()).toBe("idle");
  });

  it("cancels the recording when Escape lands before the device answers", async () => {
    await armed();
    await press();
    await escape();
    // Still opening: going idle here would re-arm the control while a recording is on its way.
    expect(phase()).toBe("opening");

    await answer();

    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
    expect(phase()).toBe("idle");
    expect(phases).not.toContain("recording");
  });

  it("lets an abort outrank a release that was recorded first", async () => {
    await armed();
    await press();
    await release();
    await escape();
    await answer();

    // The user withdrew the release; nothing may be transcribed.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
    expect(phase()).toBe("idle");
  });

  it("treats a window blur during the opening window as an abort", async () => {
    await armed();
    await press();
    await blur();
    await answer();

    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
  });

  it("refuses a second hold until the abandoned one has been released", async () => {
    await armed();
    await press();
    await escape();
    await press();

    // One `startRecording`, not two: a second native recording would collide with the first on
    // the application-wide single-recording slot.
    expect(double.calls.startRecording).toEqual(["session-a"]);

    await answer();
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
  });

  it("cancels a handle that arrives after the session changed", async () => {
    const mounted = await armed();
    await press();
    await act(async () => mounted.rerenderScope("session-b"));
    await answer();

    // Cancelled, and cancelled as the session that started it. The native side matches recording
    // and scope together, so releasing it under the new session's name would be refused and the
    // application-wide slot would stay occupied.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual([]);
    expect(screen.getByTestId("draft").textContent).toBe("");
  });

  it("hands the microphone back to the session the user switched to", async () => {
    const mounted = await armed();
    await press();
    await act(async () => mounted.rerenderScope("session-b"));
    await answer();

    // The control belongs to session B now, and session B never pressed anything.
    expect(phase()).toBe("idle");

    double.deferStartRecording();
    await press();
    await answer();
    await release();

    // Two starts, in order, each under its own session.
    expect(double.calls.startRecording).toEqual(["session-a", "session-b"]);
    // Session A's handle released exactly once, and only session B's is transcribed.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(double.calls.stopRecordingAndTranscribe).toEqual(["rec-2@session-b"]);
    expect(screen.getByTestId("failure").textContent).toBe("");
  });

  it("cancels once when the session changes twice before the handle arrives", async () => {
    const mounted = await armed();
    await press();
    await act(async () => mounted.rerenderScope("session-b"));
    await act(async () => mounted.rerenderScope("session-c"));
    await answer();

    // One release, still named for the session that asked for it.
    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    expect(phase()).toBe("idle");
    // Usable again, asked the only way that means anything: by pressing it.
    await press();
    expect(double.calls.startRecording).toEqual(["session-a", "session-c"]);
  });

  it("keeps a start rejected after a session change from raising a banner", async () => {
    const mounted = await armed();
    await press();
    await act(async () => mounted.rerenderScope("session-b"));
    await act(async () => {
      double.rejectStartRecording("MIC_PERMISSION_DENIED");
      await Promise.resolve();
    });

    // The failure belongs to a session the user has left.
    expect(screen.getByTestId("failure").textContent).toBe("");
    expect(phase()).toBe("idle");
    expect(double.calls.cancelRecording).toEqual([]);
    await press();
    expect(double.calls.startRecording).toEqual(["session-a", "session-b"]);
  });

  it("cancels a handle that arrives after the composer unmounted", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const mounted = await armed();
    await press();
    await act(async () => mounted.view.unmount());
    await answer();

    expect(double.calls.cancelRecording).toEqual(["rec-1@session-a"]);
    // No state update on an unmounted controller.
    expect(errors).not.toHaveBeenCalled();
    errors.mockRestore();
  });

  it("keeps a withdrawn hold from raising a stale failure", async () => {
    await armed();
    await press();
    await escape();
    await act(async () => {
      double.rejectStartRecording("MIC_PERMISSION_DENIED");
      await Promise.resolve();
    });

    expect(screen.getByTestId("failure").textContent).toBe("");
    expect(phase()).toBe("idle");
    expect(double.calls.cancelRecording).toEqual([]);
  });

  it("still reports a denied microphone for a hold the user did not withdraw", async () => {
    await armed();
    await press();
    await act(async () => {
      double.rejectStartRecording("MIC_PERMISSION_DENIED");
      await Promise.resolve();
    });

    expect(screen.getByTestId("failure").textContent).toBe("MIC_PERMISSION_DENIED");
    expect(phase()).toBe("idle");
    expect(phases).not.toContain("recording");
  });

  it("leaves the ordinary hold untouched", async () => {
    mount();
    await waitFor(() => expect(micButton().disabled).toBe(false));

    await press();
    await act(async () => {
      await Promise.resolve();
    });
    expect(phase()).toBe("recording");

    await release();
    await act(async () => {
      await Promise.resolve();
    });

    expect(double.calls.stopRecordingAndTranscribe).toEqual(["rec-1@session-a"]);
    expect(double.calls.cancelRecording).toEqual([]);
  });
});
