import { useCallback, useRef, useState } from "react";

import { speechSourceText } from "./draft-merge";
import {
  type LocalMediaComposerContext,
  type MicrophonePhase,
  type SpeechPhase,
} from "./local-media-composer-types";
import { useLocalMediaOperation } from "./use-local-media-operation";
import { useMicrophoneRecording } from "./use-microphone-recording";
import type { MicrophoneHoldBindings } from "./use-microphone-hold";

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

/**
 * Read-aloud, plus the hold-to-talk machine it shares a draft with.
 *
 * The recording half lives in `use-microphone-recording.ts`. It is the larger of the two by a wide
 * margin -- an opening window, an ownership model, and a session-change teardown -- and keeping it
 * here made the one file that also owns playback too long to read.
 */
export function useSpeechComposer(options: SpeechComposerOptions): SpeechComposer {
  const { context, getSelection, sttEnabled } = options;
  const [speechPhase, setSpeechPhase] = useState<SpeechPhase>("idle");
  const playbackRef = useRef<string | null>(null);
  const recording = useMicrophoneRecording(context, sttEnabled);

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
    microphonePhase: recording.microphonePhase,
    speechPhase,
    recordingElapsedMs: recording.recordingElapsedMs,
    recordingLimitReached: recording.recordingLimitReached,
    microphone: recording.microphone,
    toggleSpeech,
  };
}
