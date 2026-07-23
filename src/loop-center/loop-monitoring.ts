import { useEffect, useState } from "react";
import type { LoopEvidence, LoopRun } from "../types/loop";

export function useLoopElapsed(run: LoopRun | null) {
  const active = run?.status === "queued" || run?.status === "running";
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [active]);
  if (!run) return formatLoopDuration(0);
  const start = Date.parse(run.startedAt ?? run.createdAt);
  const end = active ? now : Date.parse(run.completedAt ?? run.updatedAt);
  return formatLoopDuration(Math.max(0, end - start));
}

export function formatLoopDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function latestLoopEvidence(run: LoopRun): LoopEvidence | null {
  return run.iterations.flatMap((iteration) => iteration.evidence).at(-1) ?? null;
}

export function latestLoopOperationEvidence(run: LoopRun): LoopEvidence | null {
  const evidence = run.iterations.flatMap((iteration) => iteration.evidence);
  for (let index = evidence.length - 1; index >= 0; index -= 1) {
    if (evidence[index].operationId) return evidence[index];
  }
  return null;
}

export function evidenceDetailNumber(evidence: LoopEvidence | undefined, key: string) {
  const value = evidence?.details?.[key];
  return typeof value === "number" ? value : null;
}
