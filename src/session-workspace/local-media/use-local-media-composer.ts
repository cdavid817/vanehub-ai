import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";

import type { LocalMediaService } from "../../services/local-media-service";
import { localMediaService as defaultService } from "../../services/runtime-local-media-client";
import type { LocalMediaEngine, LocalMediaRuntimeStatus } from "../../types/local-media";
import { localMediaErrorCodeFrom } from "./local-media-errors";
import type {
  LocalMediaComposerContext,
  LocalMediaComposerModel,
  LocalMediaFailure,
} from "./local-media-composer-types";
import { useOcrComposer } from "./use-ocr-composer";
import { useSpeechComposer } from "./use-speech-composer";

export interface LocalMediaComposerOptions {
  /** `null` when no session owns the composer; every action is disabled. */
  composerScopeId: string | null;
  getDraft: () => string;
  setDraft: (value: string) => void;
  getSelection: () => { start: number; end: number } | null;
  /** The composer textarea, when the host has one to give. Focus is never forced without it. */
  getTextArea?: () => HTMLTextAreaElement | null;
  service?: LocalMediaService;
}

/**
 * Controls that keep focus even though text just landed in the draft.
 *
 * A dialog or a menu the user opened while the engine was working is a deliberate act; yanking the
 * caret away from it would discard whatever they were part-way through. A button is different --
 * the media action itself is a button, and leaving focus there after an append means the next
 * keystroke goes nowhere useful.
 */
function holdsSubstantiveFocus(active: Element | null): boolean {
  if (!active || active === document.body) return false;
  if (active.closest("[role='dialog']")) return true;
  const tag = active.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || active.hasAttribute("contenteditable");
}

/** One engine's readiness, read without letting another engine's state participate. */
function permits(status: LocalMediaRuntimeStatus | undefined, engine: LocalMediaEngine): boolean {
  if (!status?.nativeAvailable || !status.enabled) return false;
  if (status.platformSupport === "unsupported") return false;
  return status.engines.some(
    (entry) => entry.engine === engine && entry.readiness.state === "ready",
  );
}

export function useLocalMediaComposer(
  options: LocalMediaComposerOptions,
): LocalMediaComposerModel {
  const { composerScopeId, getDraft, getSelection, getTextArea, setDraft } = options;
  const service = options.service ?? defaultService;

  const [failure, setFailure] = useState<LocalMediaFailure | null>(null);
  const [overflow, setOverflow] = useState<LocalMediaComposerModel["overflow"]>(null);

  /** Which composer scope started each in-flight operation. */
  const scopeRef = useRef<Record<string, string>>({});
  const currentScope = useRef(composerScopeId);
  const previousScope = useRef(composerScopeId);
  currentScope.current = composerScopeId;
  // Operation ownership must change with the render, before a fast user can start work in the new
  // session. Clearing it later in an effect can erase an operation the new session already began.
  if (previousScope.current !== composerScopeId) {
    previousScope.current = composerScopeId;
    scopeRef.current = {};
  }

  const status = useQuery({
    queryKey: ["local-media", "status"],
    queryFn: () => service.getStatus(),
    staleTime: 5_000,
  });

  const availability = {
    native: status.data?.nativeAvailable ?? false,
    ocr: permits(status.data, "ocr"),
    stt: permits(status.data, "stt"),
    tts: permits(status.data, "tts"),
  };

  const context = useMemo<LocalMediaComposerContext>(
    () => ({
      composerScopeId,
      service,
      getDraft,
      setDraft,
      /**
       * A result belongs to this composer only if the scope that started it is still active.
       *
       * Checking at completion, rather than capturing a draft when the work began, is what keeps a
       * transcript out of a conversation the user switched to while the model was loading.
       */
      ownsResult: (operationId) =>
        scopeRef.current[operationId] !== undefined &&
        scopeRef.current[operationId] === currentScope.current,
      remember: (operationId) => {
        if (currentScope.current) scopeRef.current[operationId] = currentScope.current;
      },
      forget: (operationId) => {
        delete scopeRef.current[operationId];
      },
      reportFailure: (engine, error) =>
        setFailure({ engine, code: localMediaErrorCodeFrom(error) }),
      reportFailureCode: (engine, code) => setFailure({ engine, code }),
      reportOverflow: (text, engine) => setOverflow({ text, engine }),
      clearFailure: () => setFailure(null),
      restoreCaret: () => {
        const element = getTextArea?.();
        if (!element || holdsSubstantiveFocus(document.activeElement)) return;
        element.focus();
        // End of the draft, not where the caret was: the appended text is what the user will edit
        // next, and a caret stranded before it reads as the append having gone somewhere else.
        element.setSelectionRange(element.value.length, element.value.length);
      },
    }),
    [composerScopeId, getDraft, getTextArea, service, setDraft],
  );

  const ocr = useOcrComposer(context);
  const speech = useSpeechComposer({
    context,
    getSelection,
    sttEnabled: availability.stt,
  });

  // Held in a ref and invoked from an effect keyed only on the scope, so the reset cannot be
  // retriggered by an unrelated identity change. Listing the callbacks as dependencies instead
  // would make every re-render look like a session switch.
  const resetForScope = useRef<() => void>(() => undefined);
  resetForScope.current = () => {
    ocr.clearReview();
    setOverflow(null);
    setFailure(null);
  };
  useEffect(() => {
    // A session switch abandons everything pending rather than letting it land somewhere new. The
    // native operations continue and clean up after themselves; what changes is that nothing here
    // will read them.
    resetForScope.current();
  }, [composerScopeId]);

  const startOcr = useCallback(() => {
    if (!availability.ocr) return;
    ocr.startOcr();
  }, [availability.ocr, ocr]);

  const toggleSpeech = useCallback(() => {
    // Stopping stays available even if readiness lapsed mid-playback; only starting is gated.
    if (!availability.tts && speech.speechPhase === "idle") return;
    speech.toggleSpeech();
  }, [availability.tts, speech]);

  return {
    availability,
    ocrPhase: ocr.ocrPhase,
    microphonePhase: speech.microphonePhase,
    speechPhase: speech.speechPhase,
    recordingElapsedMs: speech.recordingElapsedMs,
    recordingLimitReached: speech.recordingLimitReached,
    failure,
    review: ocr.review,
    overflow,
    startOcr,
    updateReviewText: ocr.updateReviewText,
    appendReviewText: ocr.appendReviewText,
    cancelReview: ocr.cancelReview,
    microphone: speech.microphone,
    toggleSpeech,
    dismissFailure: () => setFailure(null),
    dismissOverflow: () => setOverflow(null),
  };
}
