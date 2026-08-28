// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useLogFollow } from "./use-log-follow";

/**
 * Two independent reasons to stop moving the viewport, and why they must stay independent.
 *
 * A log list that always jumps to the newest row is unusable while anything is being logged — the
 * line someone is reading scrolls away mid-sentence. One that never jumps is useless for watching.
 * The distinction between "the reader pressed Pause" and "the reader scrolled away" is what makes
 * the middle ground possible, and collapsing the two is what makes it fail: resuming from a pause
 * would be cancelled by wherever the scrollbar sits, and scrolling back to the top would silently
 * un-pause someone who asked for stillness.
 */
describe("log follow", () => {
  it("follows by default, because a freshly opened list is at the newest edge", () => {
    const { result } = renderHook(() => useLogFollow());

    expect(result.current.following).toBe(true);
    expect(result.current.paused).toBe(false);
    expect(result.current.atNewestEdge).toBe(true);
  });

  it("stops following when the reader pauses, even at the newest edge", () => {
    const { result } = renderHook(() => useLogFollow());

    act(() => result.current.setPaused(true));

    expect(result.current.paused).toBe(true);
    expect(result.current.atNewestEdge).toBe(true);
    expect(result.current.following).toBe(false);
  });

  it("stops following when the reader scrolls away, without claiming they paused", () => {
    const { result } = renderHook(() => useLogFollow());

    act(() => result.current.noteViewport(false));

    expect(result.current.following).toBe(false);
    // The Pause control must not light up for this. It reports a choice, and scrolling is not one
    // the reader made through that control.
    expect(result.current.paused).toBe(false);
  });

  it("resumes following when the reader scrolls back, if they never paused", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.noteViewport(false));

    act(() => result.current.noteViewport(true));

    expect(result.current.following).toBe(true);
  });

  it("keeps a paused reader paused when they scroll back to the newest edge", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.setPaused(true));

    act(() => result.current.noteViewport(true));

    // Scrolling to the top is not a request to start moving again. If it were, a paused reader who
    // scrolled up to re-read the last few lines would be thrown straight back into a moving list.
    expect(result.current.paused).toBe(true);
    expect(result.current.following).toBe(false);
  });

  it("counts rows that arrive while the view is held still", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.setPaused(true));

    act(() => result.current.notePending(3));
    act(() => result.current.notePending(2));

    // The count is what makes not-following honest: without it the list quietly stops matching the
    // session and nothing says more exists above.
    expect(result.current.pendingCount).toBe(5);
  });

  it("reports no backlog while following, because there is nothing being withheld", () => {
    const { result } = renderHook(() => useLogFollow());

    act(() => result.current.notePending(4));

    expect(result.current.following).toBe(true);
    expect(result.current.pendingCount).toBe(0);
  });

  it("clears the backlog when the reader arrives back at the newest edge", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.noteViewport(false));
    act(() => result.current.notePending(6));
    expect(result.current.pendingCount).toBe(6);

    act(() => result.current.noteViewport(true));

    // Those rows are on screen now. Still reporting them would send the reader looking for rows
    // they are already looking at.
    expect(result.current.pendingCount).toBe(0);
  });

  it("clears the backlog when the reader un-pauses", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.setPaused(true));
    act(() => result.current.notePending(9));

    act(() => result.current.setPaused(false));

    expect(result.current.following).toBe(true);
    expect(result.current.pendingCount).toBe(0);
  });

  it("resumes from both reasons at once when the reader jumps to latest", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.setPaused(true));
    act(() => result.current.noteViewport(false));
    act(() => result.current.notePending(12));

    act(() => result.current.resumeAtNewest());

    // Jump to latest is the one action that means "put me back in the stream", so it has to clear
    // both reasons. Clearing one would leave a button that visibly does nothing.
    expect(result.current.paused).toBe(false);
    expect(result.current.atNewestEdge).toBe(true);
    expect(result.current.following).toBe(true);
    expect(result.current.pendingCount).toBe(0);
  });

  it("ignores an empty or negative arrival, which would otherwise offer a jump to nothing", () => {
    const { result } = renderHook(() => useLogFollow());
    act(() => result.current.setPaused(true));

    act(() => result.current.notePending(0));
    act(() => result.current.notePending(-2));

    expect(result.current.pendingCount).toBe(0);
  });
});
