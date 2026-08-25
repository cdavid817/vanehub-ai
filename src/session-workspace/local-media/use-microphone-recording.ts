import { useCallback, useEffect, useRef, useState } from "react";

import { appendSpeechTranscript } from "./draft-merge";
import { isInformationalOutcome } from "./local-media-errors";
import {
  MAX_DRAFT_CHARACTERS,
  type LocalMediaComposerContext,
  type MicrophonePhase,
} from "./local-media-composer-types";
import { useRecordingRelease, type RecordingOwner } from "./recording-release";
import { useLocalMediaOperation } from "./use-local-media-operation";
import { useMicrophoneHold, type MicrophoneHoldBindings } from "./use-microphone-hold";

const ELAPSED_TICK_MS = 200;

/**
 * What the user asked for while `startRecording` was still in flight.
 *
 * A hold can be released, cancelled, or abandoned before the native side has answered, and none of
 * those can be carried out yet because there is no handle to act on. `abort` outranks `finish`: a
 * user who released and then pressed Escape has withdrawn the release.
 */
type PendingOpeningAction = null | "finish" | "abort";

export interface MicrophoneRecording {
  microphonePhase: MicrophonePhase;
  recordingElapsedMs: number;
  recordingLimitReached: boolean;
  microphone: MicrophoneHoldBindings;
}

/**
 * Hold-to-talk: the opening window, the running recording, and the session that owns both.
 */
export function useMicrophoneRecording(
  context: LocalMediaComposerContext,
  sttEnabled: boolean,
): MicrophoneRecording {
  const [microphonePhase, setMicrophonePhase] = useState<MicrophonePhase>("idle");
  const [recordingElapsedMs, setRecordingElapsedMs] = useState(0);
  const [recordingLimitReached, setRecordingLimitReached] = useState(false);

  /** The running recording and the scope that started it. */
  const ownerRef = useRef<RecordingOwner | null>(null);
  /** Bumped per hold. A continuation whose attempt is no longer current owns nothing. */
  const attemptRef = useRef(0);
  const pendingRef = useRef<PendingOpeningAction>(null);
  /** True from the moment a hold begins until its `startRecording` has been settled. */
  const openingRef = useRef(false);
  const mountedRef = useRef(true);
  /** The scope on screen now, which is not always the scope a recording was born under. */
  const scopeRef = useRef(context.composerScopeId);
  scopeRef.current = context.composerScopeId;
  const { settled, release: releaseRecording } = useRecordingRelease(context, mountedRef);

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
    const owner = ownerRef.current;
    if (!owner) {
      // Released before the native side answered. Recorded rather than dropped: silently returning
      // here is what used to leave the microphone open after the user let go.
      if (openingRef.current && pendingRef.current === null) pendingRef.current = "finish";
      return;
    }
    ownerRef.current = null;
    setMicrophonePhase("finalizing");
    void (async () => {
      try {
        const handle = await context.service.stopRecordingAndTranscribe(owner);
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
    const owner = ownerRef.current;
    // Abort outranks a release recorded a moment earlier: the user withdrew it.
    pendingRef.current = "abort";
    if (!owner) {
      // Still opening. The phase stays `opening` until the handle arrives and is released --
      // going idle here would re-arm the control while a native recording is still on its way.
      return;
    }
    ownerRef.current = null;
    setMicrophonePhase("idle");
    setRecordingElapsedMs(0);
    releaseRecording(owner);
  }, [releaseRecording]);

  const beginRecording = useCallback(() => {
    const { composerScopeId, service } = context;
    if (!composerScopeId || microphonePhase !== "idle" || openingRef.current) return;
    if (!settled) return;
    context.clearFailure();
    setRecordingLimitReached(false);
    setMicrophonePhase("opening");
    attemptRef.current += 1;
    const attempt = attemptRef.current;
    const startedScope = composerScopeId;
    pendingRef.current = null;
    openingRef.current = true;

    const clearAttempt = () => {
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
        const current = attempt === attemptRef.current;
        const sameScope = scopeRef.current === startedScope;
        clearAttempt();
        if (!current || !mountedRef.current) return;
        setMicrophonePhase("idle");
        // Neither a hold the user cancelled nor one belonging to a session they have left earns a
        // banner in the session they are looking at now.
        if (!withdrawn && sameScope) context.reportFailure("stt", error);
        return;
      }

      const owner: RecordingOwner = {
        recordingId: handle.recordingId,
        composerScopeId: startedScope,
      };
      const wanted = pendingRef.current;
      const current = attempt === attemptRef.current;
      const sameScope = scopeRef.current === startedScope;
      clearAttempt();

      // Four separate questions, deliberately not collapsed into one "owned" flag: whether this
      // attempt is still the current one, whether the controller is still mounted, whether the
      // session on screen is still the one that pressed, and which scope the native recording
      // belongs to. Only the last decides how it is released.
      if (!current || !mountedRef.current || !sameScope || wanted === "abort") {
        releaseRecording(owner);
        // A session change still owes the control back to whoever is looking at it. Skipping the
        // reset because the scope moved is what left the next session unable to record at all.
        if (current && mountedRef.current) {
          setMicrophonePhase("idle");
          setRecordingElapsedMs(0);
        }
        return;
      }

      ownerRef.current = owner;
      setRecordingElapsedMs(0);
      if (wanted === "finish") {
        // Straight into the ordinary finalizing/transcribing chain, exactly as a release that
        // arrived a moment later would have done.
        finishRef.current();
        return;
      }
      setMicrophonePhase("recording");
    })();
  }, [context, microphonePhase, releaseRecording, settled]);

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
    enabled:
      sttEnabled && microphonePhase === "idle" && settled && context.composerScopeId !== null,
    held: microphonePhase === "opening" || microphonePhase === "recording",
    onBegin: beginRecording,
    onFinish: finishRecording,
    onAbort: abortRecording,
  });

  /**
   * Leaving a session ends its recording.
   *
   * The composer does not necessarily unmount when the user switches: it re-renders with a new
   * scope, so the hold machine's unmount path never runs and nothing else was ending the capture.
   * The previous scope is what the release is addressed with, because that is what the native side
   * will match.
   */
  const previousScopeRef = useRef(context.composerScopeId);
  useEffect(() => {
    const previous = previousScopeRef.current;
    previousScopeRef.current = context.composerScopeId;
    // Mount, and StrictMode's mount/unmount/remount, are not session changes.
    if (previous === context.composerScopeId) return;
    // An attempt still opening under the old scope releases its handle when it arrives.
    if (openingRef.current) pendingRef.current = "abort";
    const owner = ownerRef.current;
    if (!owner) return;
    ownerRef.current = null;
    setMicrophonePhase("idle");
    setRecordingElapsedMs(0);
    setRecordingLimitReached(false);
    releaseRecording(owner);
  }, [context.composerScopeId, releaseRecording]);

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
      // Any handle still on its way is nobody's now. The continuation reads this and releases it
      // instead of leaving a recording open behind a composer that no longer exists.
      attemptRef.current += 1;
    };
  }, []);

  return { microphonePhase, recordingElapsedMs, recordingLimitReached, microphone };
}
