// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import { useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { MAX_DRAFT_CHARACTERS, type LocalMediaComposerModel } from "./local-media-composer-types";
import {
  createLocalMediaDouble,
  ocrResult,
  readyStatus,
  stagedSource,
  transcription,
  type LocalMediaDouble,
} from "./local-media-test-double";
import { useLocalMediaComposer } from "./use-local-media-composer";

interface HarnessHandle {
  model: LocalMediaComposerModel;
  draft: string;
  setDraft: (value: string) => void;
  setScope: (value: string | null) => void;
  setSelection: (value: { start: number; end: number } | null) => void;
}

/**
 * Mounts the controller with a real draft, so "append to the *latest* draft" is observable rather
 * than asserted against a value the test itself held.
 */
function Harness({
  double,
  initialDraft = "",
  initialScope = "session-a",
  onReady,
}: {
  double: LocalMediaDouble;
  initialDraft?: string;
  initialScope?: string | null;
  onReady: (handle: HarnessHandle) => void;
}) {
  const [draft, setDraft] = useState(initialDraft);
  const [scope, setScope] = useState<string | null>(initialScope);
  const [selection, setSelection] = useState<{ start: number; end: number } | null>(null);
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const selectionRef = useRef(selection);
  selectionRef.current = selection;

  const textAreaRef = useRef<HTMLTextAreaElement | null>(null);
  const model = useLocalMediaComposer({
    composerScopeId: scope,
    getDraft: () => draftRef.current,
    setDraft,
    getSelection: () => selectionRef.current,
    getTextArea: () => textAreaRef.current,
    service: double.service,
  });

  onReady({ model, draft, setDraft, setScope, setSelection });
  return (
    <>
      <span data-testid="draft">{draft}</span>
      <textarea
        data-testid="composer"
        onChange={(event) => setDraft(event.target.value)}
        ref={textAreaRef}
        value={draft}
      />
    </>
  );
}

function mount(double: LocalMediaDouble, options: { draft?: string; scope?: string | null } = {}) {
  const handle = { current: null as HarnessHandle | null };
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const view = render(
    <QueryClientProvider client={client}>
      <Harness
        double={double}
        initialDraft={options.draft}
        initialScope={options.scope}
        onReady={(next) => {
          handle.current = next;
        }}
      />
    </QueryClientProvider>,
  );
  return {
    view,
    get model() {
      if (!handle.current) throw new Error("harness never rendered");
      return handle.current.model;
    },
    get draft() {
      if (!handle.current) throw new Error("harness never rendered");
      return handle.current.draft;
    },
    type: (value: string) => act(() => handle.current?.setDraft(value)),
    setScope: (value: string | null) => act(() => handle.current?.setScope(value)),
    setSelection: (value: { start: number; end: number } | null) =>
      act(() => handle.current?.setSelection(value)),
  };
}

/** Waits for availability to arrive from the status query before the actions are pressed. */
async function whenReady(harness: ReturnType<typeof mount>) {
  await waitFor(() => expect(harness.model.availability.ocr).toBe(true));
}

/**
 * Waits out one poll interval of the operation watcher.
 *
 * Real timers on purpose. The watcher's interval, the fake service's promises, and React's own
 * scheduling all have to interleave, and a fake clock makes the ordering between them a property
 * of the test harness rather than of the code being tested.
 */
async function tick() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 400));
  });
}

describe("useLocalMediaComposer", () => {

  describe("availability", () => {
    it("enables each action from its own engine's readiness", async () => {
      const double = createLocalMediaDouble(readyStatus(["ocr"]));
      const harness = mount(double);
      await whenReady(harness);

      // One unconfigured engine must not take the others down with it.
      expect(harness.model.availability).toEqual({ native: true, ocr: true, stt: false, tts: false });
    });

    it("disables everything when the master switch is off", async () => {
      const status = { ...readyStatus(), enabled: false };
      const harness = mount(createLocalMediaDouble(status));
      await waitFor(() => expect(harness.model.availability.native).toBe(true));

      expect(harness.model.availability.ocr).toBe(false);
      expect(harness.model.availability.stt).toBe(false);
      expect(harness.model.availability.tts).toBe(false);
    });

    it("disables everything on an unsupported platform even when the engines report ready", async () => {
      const status = { ...readyStatus(), platformSupport: "unsupported" as const };
      const harness = mount(createLocalMediaDouble(status));
      await waitFor(() => expect(harness.model.availability.native).toBe(true));

      expect(harness.model.availability.ocr).toBe(false);
    });
  });

  describe("OCR", () => {
    it("stages the picked file, recognizes it, and shows the review without touching the draft", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "existing" });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("recognized text") });
      await tick();

      expect(double.calls.startOcr).toEqual(["staged-1"]);
      expect(harness.model.review?.text).toBe("recognized text");
      // The review is where the text becomes the user's; nothing reached the draft yet.
      expect(harness.draft).toBe("existing");
    });

    it("does nothing when the picker is cancelled", async () => {
      const double = createLocalMediaDouble();
      double.setStaged(null);
      const harness = mount(double);
      await whenReady(harness);

      await act(async () => harness.model.startOcr());

      expect(double.calls.startOcr).toEqual([]);
      expect(harness.model.ocrPhase).toBe("idle");
      expect(harness.model.failure).toBeNull();
    });

    it("appends the reviewed text to the draft only after the user confirms", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "notes" });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("scanned") });
      await tick();
      act(() => harness.model.updateReviewText("scanned and edited"));
      act(() => harness.model.appendReviewText());

      expect(harness.draft).toBe("notes\n\nscanned and edited");
      expect(harness.model.review).toBeNull();
    });

    it("releases the staged copy when the review is cancelled", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double);
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("scanned") });
      await tick();
      await act(async () => harness.model.cancelReview());

      expect(double.calls.discardStaged).toEqual(["staged-1"]);
      expect(harness.model.review).toBeNull();
    });

    it("reports a recognition failure as its stable code", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double);
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.fail("op-1", "MODEL_NOT_FOUND");
      await tick();

      expect(harness.model.failure).toEqual({ engine: "ocr", code: "MODEL_NOT_FOUND" });
      expect(harness.model.ocrPhase).toBe("idle");
    });

    it("keeps the draft and offers the text separately when appending would overflow", async () => {
      const double = createLocalMediaDouble();
      const draft = "x".repeat(MAX_DRAFT_CHARACTERS - 10);
      const harness = mount(double, { draft });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("y".repeat(100)) });
      await tick();
      act(() => harness.model.appendReviewText());

      // Never truncated, never overwritten: the recognized text stays reachable.
      expect(harness.draft).toBe(draft);
      expect(harness.model.overflow).toEqual({ engine: "ocr", text: "y".repeat(100) });
    });

    it("shows an empty review rather than an error when nothing was recognized", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double);
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("") });
      await tick();

      expect(harness.model.failure).toBeNull();
      expect(harness.model.review?.text).toBe("");
    });

    it("drops a result whose session the user already left", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "first session" });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      harness.setScope("session-b");
      double.settle("op-1", { kind: "ocr", result: ocrResult("late text") });
      await tick();

      // A review dialog for a file picked in another conversation would be worse than losing it.
      expect(harness.model.review).toBeNull();
      expect(harness.draft).toBe("first session");
    });
  });

  describe("hold to talk", () => {
    it("appends the transcript to whatever the draft says when the result lands", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "" });
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await finishHold(harness);
      // Typed while the engine was still working: the transcript joins this, not the empty draft
      // that existed when the hold started.
      harness.type("typed meanwhile");
      double.settle("op-1", { kind: "stt", result: transcription("hello there") });
      await tick();

      expect(harness.draft).toBe("typed meanwhile hello there");
      expect(harness.model.microphonePhase).toBe("idle");
    });

    it("leaves the draft untouched for an empty utterance and reports no error", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "kept" });
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await finishHold(harness);
      double.settle("op-1", { kind: "stt", result: transcription("   ") });
      await tick();

      expect(harness.draft).toBe("kept");
      expect(harness.model.failure).toBeNull();
    });

    it("treats a no-speech outcome as information rather than a failure", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double);
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await finishHold(harness);
      double.fail("op-1", "NO_SPEECH_DETECTED");
      await tick();

      expect(harness.model.failure).toBeNull();
    });

    it("surfaces a denied microphone before any recording indicator appears", async () => {
      const double = createLocalMediaDouble();
      vi.mocked(double.service.startRecording).mockRejectedValueOnce(
        new Error("MIC_PERMISSION_DENIED"),
      );
      const harness = mount(double);
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);

      expect(harness.model.microphonePhase).toBe("idle");
      expect(harness.model.failure).toEqual({ engine: "stt", code: "MIC_PERMISSION_DENIED" });
    });

    it("cancels the recording and transcribes nothing when the hold is aborted", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "kept" });
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await act(async () => harness.model.microphone.onPointerCancel());

      expect(double.calls.cancelRecording).toEqual(["rec-1"]);
      expect(double.service.stopRecordingAndTranscribe).not.toHaveBeenCalled();
      expect(harness.draft).toBe("kept");
    });

    it("records that the duration ceiling was reached without discarding the transcript", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double);
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await finishHold(harness);
      double.settle("op-1", { kind: "stt", result: transcription("cut off here", true) });
      await tick();

      expect(harness.model.recordingLimitReached).toBe(true);
      expect(harness.draft).toBe("cut off here");
    });

    it("drops a transcript for a session the user already left", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "first" });
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      await beginHold(harness);
      await finishHold(harness);
      harness.setScope("session-b");
      double.settle("op-1", { kind: "stt", result: transcription("late words") });
      await tick();

      expect(harness.draft).toBe("first");
    });
  });

  describe("read aloud", () => {
    it("speaks the selection when there is one", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "alpha beta gamma" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      harness.setSelection({ start: 6, end: 10 });
      await act(async () => harness.model.toggleSpeech());

      expect(double.calls.startTts).toEqual(["beta"]);
    });

    it("speaks the whole draft when the selection is empty", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "alpha beta" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      harness.setSelection({ start: 4, end: 4 });
      await act(async () => harness.model.toggleSpeech());

      expect(double.calls.startTts).toEqual(["alpha beta"]);
    });

    it("does nothing when there is no text to read", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "   " });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      await act(async () => harness.model.toggleSpeech());

      expect(double.calls.startTts).toEqual([]);
      expect(harness.model.speechPhase).toBe("idle");
    });

    it("stops immediately when pressed again", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "speak this" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      await act(async () => harness.model.toggleSpeech());
      expect(harness.model.speechPhase).not.toBe("idle");
      await act(async () => harness.model.toggleSpeech());

      expect(double.calls.stopPlayback).toBe(1);
      expect(double.calls.cancelOperation).toEqual(["op-1"]);
    });

    it("does not report a cancellation as a failure", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "speak this" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      await act(async () => harness.model.toggleSpeech());
      double.fail("op-1", "OPERATION_CANCELLED");
      await tick();

      expect(harness.model.failure).toBeNull();
    });

    it("reports a synthesis failure with its stable code", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "speak this" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      await act(async () => harness.model.toggleSpeech());
      double.fail("op-1", "PLAYBACK_DEVICE_UNAVAILABLE");
      await tick();

      expect(harness.model.failure).toEqual({
        engine: "tts",
        code: "PLAYBACK_DEVICE_UNAVAILABLE",
      });
    });

    it("never starts on its own when the draft changes", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "typing" });
      await waitFor(() => expect(harness.model.availability.tts).toBe(true));

      harness.type("more typing");

      expect(double.calls.startTts).toEqual([]);
    });
  });

  describe("focus", () => {
    it("returns the caret to the end of the draft after an append", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "notes" });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("scanned") });
      await tick();
      act(() => harness.model.appendReviewText());

      const textarea = screen.getByTestId("composer") as HTMLTextAreaElement;
      expect(document.activeElement).toBe(textarea);
      // At the end, not where it was: the appended text is what the user edits next.
      expect(textarea.selectionStart).toBe("notes\n\nscanned".length);
    });

    it("leaves focus alone when the user has moved to another substantive control", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "notes" });
      await whenReady(harness);

      await act(async () => harness.model.startOcr());
      double.settle("op-1", { kind: "ocr", result: ocrResult("scanned") });
      await tick();

      const elsewhere = document.createElement("input");
      document.body.append(elsewhere);
      elsewhere.focus();
      act(() => harness.model.appendReviewText());

      // Pulling the caret out of an input the user deliberately moved to would discard whatever
      // they were part-way through typing there.
      expect(document.activeElement).toBe(elsewhere);
      elsewhere.remove();
    });

    it("does not force focus after a transcript when a dialog is open", async () => {
      const double = createLocalMediaDouble();
      const harness = mount(double, { draft: "" });
      await waitFor(() => expect(harness.model.availability.stt).toBe(true));

      const dialog = document.createElement("div");
      dialog.setAttribute("role", "dialog");
      const button = document.createElement("button");
      dialog.append(button);
      document.body.append(dialog);

      await beginHold(harness);
      await finishHold(harness);
      button.focus();
      double.settle("op-1", { kind: "stt", result: transcription("spoken words") });
      await tick();

      expect(harness.draft).toBe("spoken words");
      expect(document.activeElement).toBe(button);
      dialog.remove();
    });
  });

  it("clears pending review, overflow, and failure state on a session switch", async () => {
    const double = createLocalMediaDouble();
    double.setStaged(stagedSource({ stagedInputId: "staged-9" }));
    const harness = mount(double);
    await whenReady(harness);

    await act(async () => harness.model.startOcr());
    double.settle("op-1", { kind: "ocr", result: ocrResult("text") });
    await tick();
    expect(harness.model.review).not.toBeNull();

    harness.setScope("session-b");

    expect(harness.model.review).toBeNull();
    expect(harness.model.overflow).toBeNull();
    expect(harness.model.failure).toBeNull();
  });
});

async function beginHold(harness: ReturnType<typeof mount>) {
  await act(async () => {
    harness.model.microphone.onPointerDown({
      button: 0,
      pointerId: 1,
      preventDefault: () => undefined,
      currentTarget: { setPointerCapture: () => undefined },
    } as unknown as React.PointerEvent<HTMLButtonElement>);
  });
}

async function finishHold(harness: ReturnType<typeof mount>) {
  await act(async () => {
    harness.model.microphone.onPointerUp({
      pointerId: 1,
      currentTarget: { releasePointerCapture: () => undefined },
    } as unknown as React.PointerEvent<HTMLButtonElement>);
  });
}
