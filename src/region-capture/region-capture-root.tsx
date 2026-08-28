import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { localMediaService } from "../services/runtime-local-media-client";
import { captureProtocolUrl } from "./capture-protocol-url";

type Point = { x: number; y: number };
type Rect = Point & { width: number; height: number };

const MIN_SELECTION = 8;

function boundedPoint(event: React.PointerEvent<HTMLElement>): Point {
  return {
    x: Math.max(0, Math.min(window.innerWidth, event.clientX)),
    y: Math.max(0, Math.min(window.innerHeight, event.clientY)),
  };
}

function rectBetween(start: Point, end: Point): Rect {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}

export function RegionCaptureRoot() {
  const { t } = useTranslation();
  const query = useMemo(() => new URLSearchParams(window.location.search), []);
  const runId = query.get("run") ?? "";
  const displayToken = query.get("display") ?? "";
  const startRef = useRef<Point | null>(null);
  const [selection, setSelection] = useState<Rect | null>(null);
  const [busy, setBusy] = useState(false);

  const cancel = useCallback(() => {
    if (!runId || busy) return;
    setBusy(true);
    void localMediaService.cancelScreenshotSelection({ runId }).catch(() => setBusy(false));
  }, [busy, runId]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") cancel();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [cancel]);

  const onPointerDown = (event: React.PointerEvent<HTMLElement>) => {
    if (event.button === 2) {
      cancel();
      return;
    }
    if (event.button !== 0 || busy) return;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    const start = boundedPoint(event);
    startRef.current = start;
    setSelection({ ...start, width: 0, height: 0 });
  };

  const onPointerMove = (event: React.PointerEvent<HTMLElement>) => {
    if (!startRef.current || busy) return;
    setSelection(rectBetween(startRef.current, boundedPoint(event)));
  };

  const onPointerUp = (event: React.PointerEvent<HTMLElement>) => {
    const start = startRef.current;
    startRef.current = null;
    if (!start || busy) return;
    const next = rectBetween(start, boundedPoint(event));
    setSelection(next);
    if (next.width < MIN_SELECTION || next.height < MIN_SELECTION) return;
    setBusy(true);
    void localMediaService.commitScreenshotSelection({
      runId,
      displayToken,
      ...next,
    }).catch(() => setBusy(false));
  };

  return (
    <main
      aria-label={t("localMedia.capture.ariaLabel")}
      className="relative h-screen w-screen cursor-crosshair select-none overflow-hidden bg-black"
      data-testid="region-capture-surface"
      onContextMenu={(event) => event.preventDefault()}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
    >
      <img
        alt=""
        className="pointer-events-none absolute inset-0 h-full w-full object-fill"
        draggable={false}
        src={captureProtocolUrl(runId, displayToken)}
      />
      <svg aria-hidden="true" className="pointer-events-none absolute inset-0 h-full w-full">
        <defs>
          <mask id="capture-selection-mask">
            <rect fill="white" height="100%" width="100%" />
            {selection ? <rect fill="black" {...selection} /> : null}
          </mask>
        </defs>
        <rect fill="rgba(0,0,0,0.48)" height="100%" mask="url(#capture-selection-mask)" width="100%" />
        {selection ? <rect className="fill-none stroke-primary stroke-2" {...selection} /> : null}
      </svg>
      <div className="pointer-events-none absolute left-1/2 top-5 -translate-x-1/2 rounded-lg bg-background/90 px-4 py-2 text-sm text-foreground shadow-xl backdrop-blur">
        {selection && selection.width >= MIN_SELECTION && selection.height >= MIN_SELECTION
          ? t("localMedia.capture.dimensions", {
              width: Math.round(selection.width),
              height: Math.round(selection.height),
            })
          : t("localMedia.capture.hint")}
      </div>
      <button
        autoFocus
        className="absolute bottom-5 left-1/2 -translate-x-1/2 rounded-md bg-background/90 px-4 py-2 text-sm text-foreground shadow-xl hover:bg-background focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary"
        disabled={busy}
        onClick={(event) => {
          event.stopPropagation();
          cancel();
        }}
        type="button"
      >
        {t("localMedia.capture.cancel")}
      </button>
    </main>
  );
}
