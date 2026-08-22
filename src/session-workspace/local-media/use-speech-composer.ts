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

  const beginRecording = useCallback(() => {
    const { composerScopeId, service } = context;
    if (!composerScopeId || microphonePhase !== "idle") return;
    context.clearFailure();
    setRecordingLimitReached(false);
    setMicrophonePhase("opening");
    void (async () => {
      try {
        const handle = await service.startRecording({ composerScopeId });
        recordingRef.current = handle.recordingId;
        setRecordingElapsedMs(0);
        setMicrophonePhase("recording");
      } catch (error) {
        // A denied microphone surfaces here, before any recording visual appeared.
        setMicrophonePhase("idle");
        recordingRef.current = null;
        context.reportFailure("stt", error);
      }
    })();
  }, [context, microphonePhase]);

  const finishRecording = useCallback(() => {
    const recordingId = recordingRef.current;
    const { composerScopeId, service } = context;
    if (!recordingId || !composerScopeId) return;
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

  const abortRecording = useCallback(() => {
    const recordingId = recordingRef.current;
    const { composerScopeId, service } = context;
    recordingRef.current = null;
    setMicrophonePhase("idle");
    setRecordingElapsedMs(0);
    if (recordingId && composerScopeId) {
      void service.cancelRecording({ recordingId, composerScopeId }).catch(() => undefined);
    }
  }, [context]);

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
