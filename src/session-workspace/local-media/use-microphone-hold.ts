import { useCallback, useEffect, useRef } from "react";

/**
 * The press-and-hold input machine, with no knowledge of recording itself.
 *
 * Split from the controller because the interesting behaviour here is entirely about input events:
 * which gesture means "finish and transcribe" and which means "throw it away". Keeping the two
 * apart is what lets the pointer, keyboard, blur, and Escape paths be tested without a microphone.
 */
export interface MicrophoneHoldCallbacks {
  /** The user began a hold. Resolves once capture is actually running. */
  onBegin: () => void;
  /** A valid release: finish the recording and transcribe it. */
  onFinish: () => void;
  /** Anything that is not a valid release. Nothing is transcribed. */
  onAbort: () => void;
  /** False while the control is disabled; a hold must not start. */
  enabled: boolean;
  /** True while a hold is in progress, from the controller's point of view. */
  held: boolean;
}

export interface MicrophoneHoldBindings {
  onPointerDown: (event: React.PointerEvent<HTMLButtonElement>) => void;
  onPointerUp: (event: React.PointerEvent<HTMLButtonElement>) => void;
  onPointerCancel: () => void;
  onLostPointerCapture: () => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
  onKeyUp: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
  onClickCapture: (event: React.MouseEvent<HTMLButtonElement>) => void;
}

const ACTIVATION_KEYS = new Set([" ", "Spacebar", "Enter"]);

export function useMicrophoneHold(
  callbacks: MicrophoneHoldCallbacks,
): MicrophoneHoldBindings {
  const latest = useRef(callbacks);
  latest.current = callbacks;
  /** Set between a completed hold and the synthetic click the browser sends after it. */
  const suppressClick = useRef(false);
  const keyboardHold = useRef(false);

  const abort = useCallback(() => {
    keyboardHold.current = false;
    if (latest.current.held) {
      latest.current.onAbort();
    }
  }, []);

  useEffect(() => {
    // A hold that survives the window losing focus would keep the microphone open behind another
    // application, and the release that ends it would never arrive.
    const onBlur = () => abort();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") abort();
    };
    window.addEventListener("blur", onBlur);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [abort]);

  useEffect(
    () => () => {
      // Unmount during a hold -- a session switch, a route change -- cancels rather than
      // transcribing: nobody asked for the utterance to be used.
      if (latest.current.held) {
        latest.current.onAbort();
      }
    },
    [],
  );

  return {
    onPointerDown: (event) => {
      if (!latest.current.enabled || event.button !== 0) return;
      // Stops the press from starting a text selection that would survive the hold.
      event.preventDefault();
      event.currentTarget.setPointerCapture?.(event.pointerId);
      latest.current.onBegin();
    },
    onPointerUp: (event) => {
      if (!latest.current.held) return;
      event.currentTarget.releasePointerCapture?.(event.pointerId);
      suppressClick.current = true;
      latest.current.onFinish();
    },
    onPointerCancel: abort,
    onLostPointerCapture: () => {
      // Losing capture mid-hold means the gesture was interrupted -- a system gesture, a drag out
      // of the window. There is no release coming, so this is an abort rather than a finish.
      if (latest.current.held) abort();
    },
    onKeyDown: (event) => {
      // A repeat is the OS auto-repeating a key that is already down, not a second press.
      if (event.repeat) return;
      // Escape is deliberately not handled here. The keydown bubbles to the window listener above,
      // which already aborts from anywhere; handling it in both places called `onAbort` twice for
      // one press.
      if (!ACTIVATION_KEYS.has(event.key)) return;
      // An IME candidate window uses Enter and Space; starting a recording mid-composition would
      // both fail to insert the candidate and open the microphone the user never asked for.
      if (event.nativeEvent.isComposing) return;
      if (!latest.current.enabled) return;
      event.preventDefault();
      keyboardHold.current = true;
      latest.current.onBegin();
    },
    onKeyUp: (event) => {
      if (!keyboardHold.current || !ACTIVATION_KEYS.has(event.key)) return;
      event.preventDefault();
      keyboardHold.current = false;
      // The browser fires `click` after a keyboard activation too; the same suppression applies.
      suppressClick.current = true;
      if (latest.current.held) {
        latest.current.onFinish();
      }
    },
    onClickCapture: (event) => {
      if (!suppressClick.current) return;
      suppressClick.current = false;
      event.preventDefault();
      event.stopPropagation();
    },
  };
}
