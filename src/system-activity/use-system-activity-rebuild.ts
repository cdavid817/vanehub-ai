import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { SystemActivitySession } from "../services/system-activity-service";

export interface RebuildProgress {
  rebuildId: string;
  phase: "running" | "validating" | "activating" | "cancelling";
  processedItems: number;
  itemBudget: number;
}

function yieldToControls(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

export function useSystemActivityRebuild(
  session: SystemActivitySession,
  onChanged: () => void,
  report: (message: string) => void,
) {
  const { t } = useTranslation();
  const [progress, setProgress] = useState<RebuildProgress | null>(null);
  const cancelRequestedRef = useRef(false);
  const activeRebuildIdRef = useRef<string | null>(null);

  const cancelled = () => cancelRequestedRef.current;
  const updatePhase = (phase: RebuildProgress["phase"]) => {
    setProgress((current) => (current ? { ...current, phase } : null));
  };
  const updateItems = (processedItems?: number) => {
    if (processedItems === undefined) return;
    setProgress((current) => (current ? { ...current, processedItems } : null));
  };

  const run = async () => {
    if (progress) return;
    cancelRequestedRef.current = false;
    try {
      const rebuild = await agentService.beginSystemActivityRebuild(
        session.scopeKind,
        session.canonicalScopeId,
        10_000,
      );
      activeRebuildIdRef.current = rebuild.rebuildId;
      setProgress({
        rebuildId: rebuild.rebuildId,
        phase: "running",
        processedItems: rebuild.processedItems,
        itemBudget: rebuild.itemBudget,
      });
      await yieldToControls();
      let step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
      while (step.step === "running") {
        if (cancelled()) return;
        updateItems(step.processedItems);
        await yieldToControls();
        step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
      }
      if (cancelled()) return;
      updateItems(step.processedItems);
      updatePhase("validating");
      await agentService.validateSystemActivityRebuild(rebuild.rebuildId);
      if (cancelled()) return;
      updatePhase("activating");
      let activation = await agentService.activateSystemActivityRebuild(rebuild.rebuildId);
      while (activation.step === "needsCatchUp") {
        if (cancelled()) return;
        updatePhase("running");
        step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
        while (step.step === "running") {
          if (cancelled()) return;
          updateItems(step.processedItems);
          await yieldToControls();
          step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
        }
        if (cancelled()) return;
        updatePhase("validating");
        await agentService.validateSystemActivityRebuild(rebuild.rebuildId);
        if (cancelled()) return;
        updatePhase("activating");
        activation = await agentService.activateSystemActivityRebuild(rebuild.rebuildId);
      }
      report(t("systemActivity.view.rebuildDone"));
      onChanged();
    } catch (error) {
      if (!cancelled()) report(error instanceof Error ? error.message : String(error));
    } finally {
      activeRebuildIdRef.current = null;
      setProgress(null);
    }
  };

  const cancel = async () => {
    const rebuildId = activeRebuildIdRef.current;
    if (!rebuildId) return;
    cancelRequestedRef.current = true;
    updatePhase("cancelling");
    try {
      await agentService.cancelSystemActivityRebuild(rebuildId);
      report(t("systemActivity.view.rebuildCancelled"));
      onChanged();
    } catch (error) {
      report(error instanceof Error ? error.message : String(error));
    } finally {
      activeRebuildIdRef.current = null;
      setProgress(null);
    }
  };

  return { progress, run, cancel };
}
