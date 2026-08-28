import { useCallback, useState, type MutableRefObject } from "react";

import { localMediaErrorCodeFrom } from "./local-media-errors";
import type { LocalMediaComposerContext } from "./local-media-composer-types";
import type { LocalMediaErrorCode } from "../../types/local-media";

/**
 * Who owns a native recording.
 *
 * The scope travels with the recording rather than being read from the current render. The native
 * side matches a recording by id *and* by the scope that started it -- that pairing is what stops
 * one composer ending another's capture -- so a release aimed with whatever session happens to be
 * on screen is refused, and the application-wide single-recording slot stays occupied with the
 * microphone open.
 */
export interface RecordingOwner {
  recordingId: string;
  composerScopeId: string;
}

/** Outstanding releases, and the reason the microphone is unusable if one of them failed. */
export interface ReleaseState {
  pending: number;
  blockedBy: LocalMediaErrorCode | null;
}

const RELEASED: ReleaseState = { pending: 0, blockedBy: null };

/**
 * A failed release that still leaves the slot free.
 *
 * `RECORDING_NOT_FOUND` is the end state the release was trying to reach: the native side holds no
 * such recording. Any other code means the host cannot say whether the microphone is closed, and
 * reporting the control as ready on no evidence is the one answer it must not give.
 */
const HARMLESS_FAILURE: LocalMediaErrorCode = "RECORDING_NOT_FOUND";

export interface RecordingRelease {
  /** True while nothing is outstanding and nothing has failed unaccountably. */
  settled: boolean;
  /** Hand a recording back to the native side, under the scope that started it. */
  release: (owner: RecordingOwner) => void;
}

/**
 * Releasing a recording, and what an unsuccessful release means for the control.
 *
 * The rejection is not swallowed. Until a release settles the microphone stays unusable, because a
 * second hold would collide with the first on the single-recording slot; if it settles as anything
 * other than "there was no such recording", it stays unusable and says why.
 */
export function useRecordingRelease(
  context: LocalMediaComposerContext,
  mountedRef: MutableRefObject<boolean>,
): RecordingRelease {
  const [state, setState] = useState<ReleaseState>(RELEASED);

  const release = useCallback(
    (owner: RecordingOwner) => {
      setState((current) => ({ ...current, pending: current.pending + 1 }));
      void context.service
        .cancelRecording(owner)
        .then(() => null)
        .catch((error: unknown) => localMediaErrorCodeFrom(error))
        .then((code) => {
          const blocking = code && code !== HARMLESS_FAILURE ? code : null;
          if (!mountedRef.current) return;
          setState((current) => ({
            pending: Math.max(0, current.pending - 1),
            blockedBy: current.blockedBy ?? blocking,
          }));
          if (blocking) context.reportFailureCode("stt", blocking);
        });
    },
    [context, mountedRef],
  );

  return { settled: state.pending === 0 && state.blockedBy === null, release };
}
