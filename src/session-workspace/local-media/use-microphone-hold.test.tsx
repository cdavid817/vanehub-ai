// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useMicrophoneHold } from "./use-microphone-hold";

interface Calls {
  begin: number;
  finish: number;
  abort: number;
}

/**
 * A button wired exactly the way `ComposerMediaActions` wires it, including the `held` flag being
 * driven by `onBegin`. Faking `held` as a constant would make several of these tests pass for the
 * wrong reason -- most of the machine's decisions are conditioned on it.
 */
function Harness({ calls, enabled = true }: { calls: Calls; enabled?: boolean }) {
  const [held, setHeld] = useState(false);
  const bindings = useMicrophoneHold({
    enabled: enabled && !held,
    held,
    onBegin: () => {
      calls.begin += 1;
      setHeld(true);
    },
    onFinish: () => {
      calls.finish += 1;
      setHeld(false);
    },
    onAbort: () => {
      calls.abort += 1;
      setHeld(false);
    },
  });
  return (
    <button data-held={held} data-testid="hold" type="button" {...bindings}>
      hold
    </button>
  );
}

function freshCalls(): Calls {
  return { begin: 0, finish: 0, abort: 0 };
}

function pointerDown(element: HTMLElement, button = 0) {
  fireEvent.pointerDown(element, { button, pointerId: 1 });
}

function pointerUp(element: HTMLElement) {
  fireEvent.pointerUp(element, { button: 0, pointerId: 1 });
}

describe("useMicrophoneHold", () => {
  beforeEach(() => {
    // jsdom implements neither method; the machine calls both and must not depend on them.
    Element.prototype.setPointerCapture = vi.fn();
    Element.prototype.releasePointerCapture = vi.fn();
  });

  it("begins on pointer down and finishes on pointer up", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    pointerDown(button);
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 0 });
    pointerUp(button);
    expect(calls).toEqual({ begin: 1, finish: 1, abort: 0 });
  });

  it("captures the pointer so a release outside the button still finishes the hold", () => {
    const capture = vi.fn();
    Element.prototype.setPointerCapture = capture;
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    pointerDown(screen.getByTestId("hold"));
    expect(capture).toHaveBeenCalledWith(1);
  });

  it("ignores a non-primary button so a right-click never opens the microphone", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    pointerDown(screen.getByTestId("hold"), 2);
    expect(calls.begin).toBe(0);
  });

  it("does not begin while disabled", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} enabled={false} />);

    pointerDown(screen.getByTestId("hold"));
    fireEvent.keyDown(screen.getByTestId("hold"), { key: " " });
    expect(calls.begin).toBe(0);
  });

  it("suppresses the synthetic click that follows a completed hold", () => {
    const calls = freshCalls();
    const onClick = vi.fn();
    render(
      <div onClick={onClick}>
        <Harness calls={calls} />
      </div>,
    );
    const button = screen.getByTestId("hold");

    pointerDown(button);
    pointerUp(button);
    fireEvent.click(button);
    // Without suppression the composer would see a click for every hold, and a click handler added
    // later would fire once per utterance.
    expect(onClick).not.toHaveBeenCalled();
  });

  it("aborts rather than finishes on pointer cancel", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    pointerDown(button);
    fireEvent.pointerCancel(button);
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });

  it("aborts when pointer capture is lost mid-hold", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    pointerDown(button);
    fireEvent.lostPointerCapture(button);
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });

  it("aborts when the window loses focus during a hold", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    pointerDown(screen.getByTestId("hold"));
    fireEvent.blur(window);
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });

  it("aborts on Escape from anywhere in the window", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    pointerDown(screen.getByTestId("hold"));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });

  it("does not abort on blur when no hold is in progress", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    fireEvent.blur(window);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(calls).toEqual({ begin: 0, finish: 0, abort: 0 });
  });

  it("aborts a hold that is still active when the control unmounts", () => {
    const calls = freshCalls();
    const view = render(<Harness calls={calls} />);

    pointerDown(screen.getByTestId("hold"));
    view.unmount();
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });

  it.each([" ", "Enter"])("holds through a %s keydown and finishes on keyup", (key) => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    fireEvent.keyDown(button, { key });
    expect(calls.begin).toBe(1);
    fireEvent.keyUp(button, { key });
    expect(calls).toEqual({ begin: 1, finish: 1, abort: 0 });
  });

  it("treats an auto-repeat as part of the same press", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    fireEvent.keyDown(button, { key: " " });
    fireEvent.keyDown(button, { key: " ", repeat: true });
    fireEvent.keyDown(button, { key: " ", repeat: true });
    expect(calls.begin).toBe(1);
  });

  it("does not start while an IME composition is active", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    fireEvent.keyDown(screen.getByTestId("hold"), { key: "Enter", isComposing: true });
    // Enter during composition commits a candidate. Recording then would both swallow the
    // candidate and open a microphone nobody asked for.
    expect(calls.begin).toBe(0);
  });

  it("ignores a keyup for a key that never started a hold", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);

    fireEvent.keyUp(screen.getByTestId("hold"), { key: " " });
    expect(calls).toEqual({ begin: 0, finish: 0, abort: 0 });
  });

  it("does not finish a keyboard hold that Escape already aborted", () => {
    const calls = freshCalls();
    render(<Harness calls={calls} />);
    const button = screen.getByTestId("hold");

    fireEvent.keyDown(button, { key: " " });
    fireEvent.keyDown(button, { key: "Escape" });
    fireEvent.keyUp(button, { key: " " });
    expect(calls).toEqual({ begin: 1, finish: 0, abort: 1 });
  });
});
