import { useCallback, useRef, useState } from "react";

import type { StagedOcrSource } from "../../types/local-media";
import { appendOcrText } from "./draft-merge";
import {
  MAX_DRAFT_CHARACTERS,
  type LocalMediaComposerContext,
  type OcrPhase,
  type OcrReviewState,
} from "./local-media-composer-types";
import { useLocalMediaOperation } from "./use-local-media-operation";

export interface OcrComposer {
  ocrPhase: OcrPhase;
  review: OcrReviewState | null;
  startOcr: () => void;
  startScreenshot: () => void;
  updateReviewText: (text: string) => void;
  appendReviewText: () => void;
  cancelReview: () => void;
  clearReview: () => void;
}

/**
 * Pick, stage, recognize, review, append.
 *
 * The review step is not a confirmation dialog bolted on for safety -- it is where the text becomes
 * the user's. OCR output is frequently imperfect, and inserting it directly would make the composer
 * a place where edits are undone rather than made.
 */
export function useOcrComposer(context: LocalMediaComposerContext): OcrComposer {
  const [ocrPhase, setOcrPhase] = useState<OcrPhase>("idle");
  const [review, setReview] = useState<OcrReviewState | null>(null);
  const stagedRef = useRef<StagedOcrSource | null>(null);

  const operation = useLocalMediaOperation(context.service, (outcome, operationId) => {
    const owned = context.ownsResult(operationId);
    context.forget(operationId);
    setOcrPhase("idle");
    const staged = stagedRef.current;
    stagedRef.current = null;

    if (outcome.status === "failed") {
      if (owned) context.reportFailureCode("ocr", outcome.code);
      return;
    }
    // A result for a session the user has left is dropped rather than shown: the review dialog
    // would appear over a conversation that has nothing to do with the file they picked.
    if (!owned || outcome.result.kind !== "ocr" || !staged) return;
    setReview({
      source: staged,
      result: outcome.result.result,
      text: outcome.result.result.plainText,
    });
  });

  const startFrom = useCallback((source: "file" | "screenshot") => {
    const { composerScopeId, service } = context;
    if (!composerScopeId || ocrPhase !== "idle") return;
    context.clearFailure();
    setOcrPhase("picking");
    void (async () => {
      try {
        const staged = source === "file"
          ? await service.selectAndStageOcrSource()
          : await service.selectAndStageScreenshotRegion({ composerScopeId });
        if (!staged) {
          // Closing the picker is not a failure and must not start anything.
          setOcrPhase("idle");
          return;
        }
        stagedRef.current = staged;
        setOcrPhase("running");
        const handle = await service.startOcr({
          stagedInputId: staged.stagedInputId,
          composerScopeId,
        });
        context.remember(handle.operationId);
        operation.watch(handle.operationId);
      } catch (error) {
        setOcrPhase("idle");
        stagedRef.current = null;
        context.reportFailure("ocr", error);
      }
    })();
  }, [context, ocrPhase, operation]);

  const startOcr = useCallback(() => startFrom("file"), [startFrom]);
  const startScreenshot = useCallback(() => startFrom("screenshot"), [startFrom]);

  const appendReviewText = useCallback(() => {
    if (!review) return;
    const next = appendOcrText(context.getDraft(), review.text);
    if (next.length > MAX_DRAFT_CHARACTERS) {
      // Never truncate and never overwrite. The reviewed text stays reachable so the user can copy
      // it somewhere the limit does not apply.
      context.reportOverflow(review.text, "ocr");
      return;
    }
    context.setDraft(next);
    setReview(null);
    context.restoreCaret();
  }, [context, review]);

  const cancelReview = useCallback(() => {
    const staged = review?.source.stagedInputId;
    setReview(null);
    // Releasing the staged copy immediately rather than waiting for its expiry: the user has said
    // they do not want it.
    if (staged) {
      void context.service.discardStagedOcrSource(staged).catch(() => undefined);
    }
  }, [context.service, review]);

  const updateReviewText = useCallback(
    (text: string) => setReview((current) => (current ? { ...current, text } : null)),
    [],
  );
  // Stable by construction. The controller uses `clearReview` as an effect dependency, and a fresh
  // identity per render turned "reset when the session changes" into "reset on every render",
  // which discarded the review dialog and the operation-ownership map between one keystroke and
  // the next.
  const clearReview = useCallback(() => setReview(null), []);

  return {
    ocrPhase,
    review,
    startOcr,
    startScreenshot,
    updateReviewText,
    appendReviewText,
    cancelReview,
    clearReview,
  };
}
