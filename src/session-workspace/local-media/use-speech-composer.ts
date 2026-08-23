import { useCallback, useEffect, useRef, useState } from "react";

import { appendSpeechTranscript, speechSourceText } from "./draft-merge";
import { isInformationalOutcome } from "./local-media-errors";
import {
  MAX_DRAFT_CHARACTERS,
  type LocalMediaComposerContext,
  type MicrophonePhase,
  type SpeechPhase,
} from "./local-media-composer-types";
import { useLocalMediaOperation } from "./use-local-media-operation";
import { useMicrophoneHold, type MicrophoneHoldBindings } from "./use-microphone-hold";

const ELAPSED_TICK_MS = 200;

/**
 * What the user asked for while `startRecording` was still in flight.
 *
 * A hold can be released, cancelled, or abandoned before the native side has answered, and none of
 * those instructions can be carried out yet because there is no recording handle to act on. They
 * are recorded here and applied the moment the handle arrives. `abort` outranks `finish`: a user
 * who released and then pressed Escape has withdrawn the release.
 */
type PendingOpeningAction = null | "finish" | "abort";

export interface SpeechComposer {
  microphonePhase: MicrophonePhase;
  speechPhase: SpeechPhase;
  recordingElapsedMs: number;
  recordingLimitReached: boolean;
  microphone: MicrophoneHoldBindings;
  toggleSpeech: () => void;
}

export interface SpeechComposerOptions {
  context: LocalMediaComposerContext;
  sttEnabled: boolean;
  getSelection: () => { start: number; end: number } | null;
}

/** Hold-to-talk and read-aloud. They share a hook because they share the one draft they write to. */
export function useSpeechComposer(options: SpeechComposerOptions): SpeechComposer {
  const { context, getSelection, sttEnabled } = options;
  const [microphonePhase, setMicrophonePhase] = useState<MicrophonePhase>("idle");
  const [speechPhase, setSpeechPhase] = useState<SpeechPhase>("idle");
  const [recordingElapsedMs, setRecordingElapsedMs] = useState(0);
  const [recordingLimitReached, setRecordingLimitReached] = useState(false);
  const recordingRef = useRef<string | null>(null);
  const playbackRef = useRef<string | null>(null);
  /** Bumped per hold. A continuation whose attempt is no longer current owns nothing. */
  const attemptRef = useRef(0);
  const pendingRef = useRef<PendingOpeningAction>(null);
  /** True from the moment a hold begins until its `startRecording` has been settled. */
  const openingRef = useRef(false);
  const mountedRef = useRef(true);
  const scopeRef = useRef(context.composerScopeId);
  scopeRef.current = context.composerScopeId;

  const transcription = useLocalMediaOperation(context.service, (outcome, operationId) => {
    const owned = context.ownsResult(operationId);
    context.forget(operationId);
    setMicrophonePhase("idle");

    if (outcome.status === "failed") {
      // An empty utterance is an outcome, not a fault. The draft is untouched either way, but only
      // a real failure earns an error affordance.
      if (owned && !isInformationalOutcome(outcome.code)) {
        context.reportFailureCode("stt", outcome.code);
      }
      return;
    }
    if (!owned || outcome.result.kind !== "stt") return;

    const result = outcome.result.result;
    setRecordingLimitReached(result.limitReached);
    if (!result.text.trim()) return;
    // The draft is read here, not when the hold started: whatever the user typed while the model
    // was loading is part of it.
    const next = appendSpeechTranscript(context.getDraft(), result.text);
    if (next.length > MAX_DRAFT_CHARACTERS) {
      context.reportOverflow(result.text, "stt");
      return;
    }
    context.setDraft(next);
    context.restoreCaret();
  });

  const finishRecording = useCallback(() => {
    const recordingId = recordingRef.current;
    const { composerScopeId, service } = context;
    if (!composerScopeId) return;
    if (!recordingId) {
      // Released before the native side answered. Recorded rather than dropped: silently
      // returning here is what used to leave the microphone open after the user let go.
      if (openingRef.current && pendingRef.current === null) pendingRef.current = "finish";
      return;
    }
    recordingRef.current = null;
    setMicrophonePhase("finalizing");
    void (async () => {
      try {
        const handle = await service.stopRecordingAndTranscribe({
          recordingId,
          composerScopeId,
        });
        context.remember(handle.operationId);
        setMicrophonePhase("transcribing");
        transcription.watch(handle.operationId);
      } catch (error) {
        setMicrophonePhase("idle");
        context.reportFailure("stt", error);
      }
    })();
  }, [context, transcription]);
  // Read by the opening continuation, which must not depend on this callback's identity.
  const finishRef = useRef(finishRecording);
  finishRef.current = finishRecording;

  const abortRecording = useCallback(() => {
    const recordingId = recordingRef.current;
    const { composerScopeId, service } = context;
    // Abort outranks a release recorded a moment earlier: the user withdrew it.
    pendingRef.current = "abort";
    if (!recordingId) {
      // Still opening. The phase stays `opening` until the handle arrives and is cancelled --
      // going idle here would re-arm the control while a native recording is still on its way,
      // and the two would collide on the application-wide single-recording slot.
      return;
    }
    recordingRef.current = null;
    setMicrophonePhase("idle");
    setRecordingElapsedMs(0);
    if (composerScopeId) {
      void service.cancelRecording({ recordingId, composerScopeId }).catch(() => undefined);
    }
  }, [context]);

  const beginRecording = useCallback(() => {
    const { composerScopeId, service } = context;
    if (!composerScopeId || microphonePhase !== "idle" || openingRef.current) return;
    context.clearFailure();
    setRecordingLimitReached(false);
    setMicrophonePhase("opening");
    attemptRef.current += 1;
    const attempt = attemptRef.current;
    const startedScope = composerScopeId;
    pendingRef.current = null;
    openingRef.current = true;

    /** Whether this attempt still owns the control. A newer hold, an unmount, or a session
     * switch all mean the answer that just arrived belongs to nobody. */
    const stillOurs = () =>
      attempt === attemptRef.current && mountedRef.current && scopeRef.current === startedScope;
    const release = () => {
      if (attempt === attemptRef.current) {
        openingRef.current = false;
        pendingRef.current = null;
      }
    };

    void (async () => {
      let handle;
      try {
        handle = await service.startRecording({ composerScopeId: startedScope });
      } catch (error) {
        const withdrawn = pendingRef.current === "abort";
        const owned = stillOurs();
        release();
        if (!owned) return;
        recordingRef.current = null;
        setMicrophonePhase("idle");
        // A hold the user already cancelled must not raise a banner about it.
        if (!withdrawn) context.reportFailure("stt", error);
        return;
      }

      const wanted = attempt === attemptRef.current ? pendingRef.current : "abort";
      const owned = stillOurs();
      release();

      if (!owned || wanted === "abort") {
        // The recording exists natively whether or not anyone still wants it, and the slot only
        // frees up when it is released.
        void service
          .cancelRecording({ recordingId: handle.recordingId, composerScopeId: startedScope })
          .catch(() => undefined);
        if (owned) {
          recordingRef.current = null;
          setMicrophonePhase("idle");
          setRecordingElapsedMs(0);
        }
        return;
      }

      recordingRef.current = handle.recordingId;
      setRecordingElapsedMs(0);
      if (wanted === "finish") {
        // Straight into the ordinary finalizing/transcribing chain, exactly as a release that
        // arrived a moment later would have done.
        finishRef.current();
        return;
      }
      setMicrophonePhase("recording");
    })();
  }, [context, microphonePhase]);

  useEffect(() => {
    if (microphonePhase !== "recording") return undefined;
    const started = Date.now();
    const timer = window.setInterval(
      () => setRecordingElapsedMs(Date.now() - started),
      ELAPSED_TICK_MS,
    );
    return () => window.clearInterval(timer);
  }, [microphonePhase]);

  const microphone = useMicrophoneHold({
    enabled: sttEnabled && microphonePhase === "idle" && context.composerScopeId !== null,
    held: microphonePhase === "opening" || microphonePhase === "recording",
    onBegin: beginRecording,
    onFinish: finishRecording,
    onAbort: abortRecording,
  });

  // Declared after the hold machine so its own unmount cleanup runs first: that cleanup aborts a
  // hold in progress, and it has to see a still-mounted controller to record the intent.
  //
  // The flag is set on every run rather than only cleared on teardown, because StrictMode mounts,
  // tears down, and remounts. Clearing without restoring left the controller permanently marked
  // unmounted in development, and every recording it started was then cancelled on arrival.
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      // Any handle still on its way is nobody's now. The continuation reads this and cancels it
      // instead of leaving a recording open behind a composer that no longer exists.
      attemptRef.current += 1;
    };
  }, []);

  const playback = useLocalMediaOperation(context.service, (outcome, operationId) => {
    const owned = context.ownsResult(operationId);
    context.forget(operationId);
    setSpeechPhase("idle");
    playbackRef.current = null;
    if (outcome.status === "failed") {
      // Stopping is a deliberate user action, not something to report back as an error.
      if (owned && outcome.code !== "OPERATION_CANCELLED") {
        context.reportFailureCode("tts", outcome.code);
      }
      return;
    }
    if (outcome.result.kind === "tts") {
      playbackRef.current = outcome.result.result.playbackId;
    }
  });

  const toggleSpeech = useCallback(() => {
    const { composerScopeId, service } = context;
    if (!composerScopeId) return;
    if (speechPhase !== "idle") {
      // One click stops whatever stage it is in: generation is cancelled through the operation,
      // playback through the sink, and the user does not have to know which is running.
      void service.stopPlayback({ playbackId: playbackRef.current ?? undefined }).catch(
        () => undefined,
      );
      const operationId = playback.operationId;
      if (operationId) void service.cancelOperation(operationId).catch(() => undefined);
      return;
    }
    const text = speechSourceText(context.getDraft(), getSelection());
    if (!text) return;
    context.clearFailure();
    setSpeechPhase("generating");
    void (async () => {
      try {
        const handle = await service.startTts({ text, composerScopeId });
        context.remember(handle.operationId);
        setSpeechPhase("playing");
        playback.watch(handle.operationId);
      } catch (error) {
        setSpeechPhase("idle");
        context.reportFailure("tts", error);
      }
    })();
  }, [context, getSelection, playback, speechPhase]);

  return {
    microphonePhase,
    speechPhase,
    recordingElapsedMs,
    recordingLimitReached,
    microphone,
    toggleSpeech,
  };
}
