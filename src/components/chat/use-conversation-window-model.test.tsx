// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { useConversationWindowModel } from "./use-conversation-window-model";

/** A real, rendered host — the hook's imperative pieces (scrollRef, item refs) need a real DOM node. */
function Host({ items }: { items: string[] }) {
  const model = useConversationWindowModel(items);
  return (
    <div>
      <div data-testid="scroll-container" onScroll={model.onScroll} ref={model.scrollRef}>
        {items.map((item) => (
          <div data-testid={`item-${item}`} key={item} ref={model.registerItemRef(item)} tabIndex={-1}>
            {item}
          </div>
        ))}
      </div>
      <span data-testid="auto-scroll-value">{String(model.autoScroll)}</span>
      <button data-testid="scroll-to-bottom" onClick={model.scrollToBottom} type="button" />
      <button data-testid="scroll-to-first" onClick={() => model.scrollToKey(items[0])} type="button" />
      <button data-testid="focus-first" onClick={() => model.focusKey(items[0])} type="button" />
    </div>
  );
}

/** jsdom leaves scrollHeight/scrollTop/clientHeight at 0 and read-only — fake them like a real scroll state. */
function stubScrollGeometry(element: HTMLElement, values: { scrollHeight: number; scrollTop: number; clientHeight: number }) {
  Object.defineProperty(element, "scrollHeight", { configurable: true, value: values.scrollHeight });
  Object.defineProperty(element, "scrollTop", { configurable: true, value: values.scrollTop, writable: true });
  Object.defineProperty(element, "clientHeight", { configurable: true, value: values.clientHeight });
}

describe("useConversationWindowModel", () => {
  beforeAll(() => {
    // jsdom does not implement ResizeObserver; this repo's convention (shell-tab.test.tsx) is a
    // no-op stub — the anchor math it would drive is already covered directly via
    // anchoredScrollTop's own tests (chat-experience.test.tsx), this file covers the wiring around it.
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  });

  it("starts near-bottom (following) by default", () => {
    render(<Host items={["a", "b"]} />);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("true");
  });

  it("stops following once a scroll leaves it more than the near-bottom threshold from the end", () => {
    render(<Host items={["a", "b"]} />);
    const container = screen.getByTestId("scroll-container");
    stubScrollGeometry(container, { clientHeight: 400, scrollHeight: 2000, scrollTop: 300 });
    fireEvent.scroll(container);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("false");
  });

  it("resumes following once a scroll lands back within the near-bottom threshold", () => {
    render(<Host items={["a", "b"]} />);
    const container = screen.getByTestId("scroll-container");
    stubScrollGeometry(container, { clientHeight: 400, scrollHeight: 2000, scrollTop: 300 });
    fireEvent.scroll(container);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("false");
    stubScrollGeometry(container, { clientHeight: 400, scrollHeight: 2000, scrollTop: 1620 });
    fireEvent.scroll(container);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("true");
  });

  it("jumps to the bottom and resumes following when scrollToBottom is invoked", () => {
    render(<Host items={["a", "b"]} />);
    const container = screen.getByTestId("scroll-container") as HTMLDivElement;
    // Leave near-bottom first, matching the real "new messages" control's precondition.
    stubScrollGeometry(container, { clientHeight: 400, scrollHeight: 2000, scrollTop: 300 });
    fireEvent.scroll(container);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("false");

    Object.defineProperty(container, "scrollHeight", { configurable: true, value: 2400 });
    fireEvent.click(screen.getByTestId("scroll-to-bottom"));
    expect(container.scrollTop).toBe(2400);
    expect(screen.getByTestId("auto-scroll-value").textContent).toBe("true");
  });

  it("scrolls a registered item into view by its stable key, for selected-item restoration", () => {
    render(<Host items={["first", "second"]} />);
    const item = screen.getByTestId("item-first");
    const scrollIntoView = vi.fn();
    item.scrollIntoView = scrollIntoView;
    fireEvent.click(screen.getByTestId("scroll-to-first"));
    expect(scrollIntoView).toHaveBeenCalledWith({ block: "nearest" });
  });

  it("moves keyboard focus to a registered item by its stable key", () => {
    render(<Host items={["first", "second"]} />);
    const item = screen.getByTestId("item-first");
    const focus = vi.fn();
    item.focus = focus;
    fireEvent.click(screen.getByTestId("focus-first"));
    expect(focus).toHaveBeenCalledTimes(1);
  });

  it("does nothing for a key that was never registered, rather than throwing", () => {
    render(<Host items={["first"]} />);
    expect(() => fireEvent.click(screen.getByTestId("scroll-to-first"))).not.toThrow();
  });

  it("stops observing its container on unmount", () => {
    const disconnect = vi.spyOn(ResizeObserver.prototype, "disconnect");
    const { unmount } = render(<Host items={["a"]} />);
    unmount();
    expect(disconnect).toHaveBeenCalled();
  });
});
