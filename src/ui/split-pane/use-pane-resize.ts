import { useCallback, useEffect, useRef, useState } from "react";

const KEY_STEP = 16;
const KEY_STEP_LARGE = 64;

export interface UsePaneResizeOptions {
  direction: "row" | "column";
  growsWithPointer: 1 | -1;
  size: number;
  min: number;
  max: number;
  onSizeChange: (size: number) => void;
  onResizeEnd?: (size: number) => void;
}

export function clampSize(size: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, size));
}

/** Shared pointer-drag and keyboard-resize behavior for a splitter/gutter element. */
export function usePaneResize({ direction, growsWithPointer, size, min, max, onSizeChange, onResizeEnd }: UsePaneResizeOptions) {
  const [isDragging, setIsDragging] = useState(false);
  const dragOrigin = useRef<{ pointer: number; size: number } | null>(null);

  useEffect(() => {
    if (!isDragging) return;

    // Derived from the pointer origin captured at drag-start, not from the `size` prop, which
    // would be stale here: `onSizeChange` updates the caller's state asynchronously, so this
    // closure never observes the latest value mid-drag.
    function nextSizeFor(event: PointerEvent): number | null {
      const origin = dragOrigin.current;
      if (!origin) return null;
      const pointer = direction === "row" ? event.clientX : event.clientY;
      const delta = (pointer - origin.pointer) * growsWithPointer;
      return clampSize(origin.size + delta, min, max);
    }

    function handleMove(event: PointerEvent) {
      const next = nextSizeFor(event);
      if (next !== null) onSizeChange(next);
    }
    function handleUp(event: PointerEvent) {
      const next = nextSizeFor(event);
      setIsDragging(false);
      dragOrigin.current = null;
      if (next === null) return;
      onSizeChange(next);
      onResizeEnd?.(next);
    }

    document.addEventListener("pointermove", handleMove);
    document.addEventListener("pointerup", handleUp);
    return () => {
      document.removeEventListener("pointermove", handleMove);
      document.removeEventListener("pointerup", handleUp);
    };
  }, [isDragging, direction, growsWithPointer, min, max, onSizeChange, onResizeEnd]);

  const handlePointerDown = useCallback((event: React.PointerEvent) => {
    dragOrigin.current = { pointer: direction === "row" ? event.clientX : event.clientY, size };
    setIsDragging(true);
  }, [direction, size]);

  const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
    const forward = direction === "row" ? "ArrowRight" : "ArrowDown";
    const backward = direction === "row" ? "ArrowLeft" : "ArrowUp";
    const step = event.shiftKey ? KEY_STEP_LARGE : KEY_STEP;
    let next: number | null = null;
    if (event.key === forward) next = size + step;
    else if (event.key === backward) next = size - step;
    else if (event.key === "Home") next = min;
    else if (event.key === "End") next = max;
    if (next === null) return;
    event.preventDefault();
    const clamped = clampSize(next, min, max);
    onSizeChange(clamped);
    onResizeEnd?.(clamped);
  }, [direction, size, min, max, onSizeChange, onResizeEnd]);

  return { isDragging, handlePointerDown, handleKeyDown };
}
