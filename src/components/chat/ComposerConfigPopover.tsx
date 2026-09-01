import { useEffect, useRef, useState, type ReactNode } from "react";
import { SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ModelInfo, ReasoningDepth, SessionExecutionMode } from "../../types/chat";
import { Button } from "../ui/button";
import { ConfigField, ConfigToggle } from "./ConfigField";
import { effectivePolicyForDisplay, isHighRiskExecutionPolicy } from "./composer-risk";
import type { RunConfigurationOverrides } from "./hooks/useRunConfigurationOverrides";
import { ConfigSelect, ModeSelect, ModelSelect, ProviderSelect, ReasoningSelect } from "./selectors";

type OpenField = "agent" | "provider" | "model" | "mode" | "reasoning" | null;

/**
 * Advanced configuration Popover (design.md Decision 9): everything `ButtonArea`'s compact
 * summary doesn't show, grouped exactly as the design calls for, with every field carrying its
 * own provenance (`ConfigField`) instead of the old always-visible five-dropdown row. Modeled on
 * `FilterPopover`'s trigger+panel shell — outside-pointerdown/Escape close, refocus the trigger —
 * not `Sheet`, since design.md defers narrow-viewport behavior to the unscheduled §20.
 */
export function ComposerConfigPopover({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  lockRuntimeIdentity,
  runConfig,
  runnerSelector,
}: {
  agents: AgentRegistryEntry[];
  availableModes: SessionExecutionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  lockRuntimeIdentity?: boolean;
  runConfig: RunConfigurationOverrides;
  /** The runner selector slot (mirrors `mediaActions`): `ApiSessionComposer` keeps owning
   *  `useRunnerSelection`, so this stays a display-only insertion point rather than a reverse
   *  dependency from `components/chat` back into `session-workspace`. */
  runnerSelector?: ReactNode;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [openField, setOpenFieldState] = useState<OpenField>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const { effectiveConfig, hasOverrides, resetAllOverrides, resetOverride, setOverride, sourceOf } = runConfig;

  // Escape is a keyboard action with nowhere else for focus to land, so it restores focus to
  // the trigger; an outside pointerdown is the user already choosing a different target, so it
  // closes without fighting that click — the same asymmetry FilterPopover uses.
  function close() {
    setOpen(false);
    setOpenFieldState(null);
    triggerRef.current?.focus();
  }

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      if (containerRef.current?.contains(event.target as Node)) return;
      setOpen(false);
      setOpenFieldState(null);
    }
    // Document-level, not a local `onKeyDown` on the panel: selecting an option in one of the
    // nested selector dropdowns removes the focused menu item from the DOM, and focus then
    // reverts to `document.body` -- outside the panel, so a bubble-based handler on the panel
    // itself would never see the Escape that follows (SelectorDropdown's own Escape handling
    // already uses the same document-level pattern for the same reason).
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") close();
    }
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
     
  }, [open]);

  const toggleField = (id: OpenField) => setOpenFieldState((current) => (current === id ? null : id));
  const closeField = () => setOpenFieldState(null);
  const agentLabel = agents.find((agent) => agent.id === effectiveConfig.agentId)?.displayName ?? effectiveConfig.agentId;
  // Re-derived for the same reason `ButtonArea`'s summary re-derives it: the profile's own
  // `effectiveExecutionPolicy` goes stale the moment `executionMode` is staged as an override.
  const displayedPolicy = effectivePolicyForDisplay(effectiveConfig);
  const highRisk = isHighRiskExecutionPolicy(displayedPolicy);

  return (
    <div className="relative" ref={containerRef}>
      <button
        aria-expanded={open}
        aria-haspopup="true"
        className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-background hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        data-testid="composer-config-trigger"
        onClick={() => setOpen((value) => !value)}
        ref={triggerRef}
        title={t("chat.config.runConfiguration")}
        type="button"
      >
        <SlidersHorizontal className="h-3.5 w-3.5" aria-hidden="true" />
        <span className="sr-only">{t("chat.config.runConfiguration")}</span>
      </button>
      {open ? (
        <div
          aria-labelledby="composer-config-popover-heading"
          className="ucd-raised absolute bottom-full left-0 z-40 mb-2 grid max-h-[70vh] w-80 max-w-[calc(100vw-2rem)] gap-3 overflow-y-auto rounded-md border border-border p-3 shadow-xl"
          data-testid="composer-config-popover"
        >
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-semibold" id="composer-config-popover-heading">{t("chat.config.runConfiguration")}</span>
            {hasOverrides ? (
              <Button className="h-7 px-2 text-xs" onClick={resetAllOverrides} size="sm" variant="outline">
                {t("chat.config.resetAll")}
              </Button>
            ) : null}
          </div>

          <section className="grid gap-1.5">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.agentRunner")}</h3>
            <ConfigField label={t("chat.config.agentLabel")} onReset={() => resetOverride("agentId")} source={sourceOf("agentId")}>
              <span className="text-xs">{agentLabel}</span>
              <ConfigSelect
                agents={agents}
                longContext={effectiveConfig.longContext}
                onAgentChange={(value) => setOverride("agentId", value)}
                onClose={closeField}
                onLongContextChange={(value) => setOverride("longContext", value)}
                onOpen={() => toggleField("agent")}
                onStreamingChange={(value) => setOverride("streaming", value)}
                onThinkingChange={(value) => setOverride("thinking", value)}
                open={openField === "agent"}
                selectedAgentId={effectiveConfig.agentId}
                streaming={effectiveConfig.streaming}
                thinking={effectiveConfig.thinking}
              />
            </ConfigField>
            {runnerSelector}
          </section>

          <section className="grid gap-1.5">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.providerModel")}</h3>
            <ConfigField label={t("chat.config.providerLabel")} onReset={() => resetOverride("providerId")} source={sourceOf("providerId")}>
              <ProviderSelect
                disabled={lockRuntimeIdentity}
                onChange={(value) => setOverride("providerId", value)}
                onClose={closeField}
                onOpen={() => toggleField("provider")}
                open={openField === "provider"}
                value={effectiveConfig.providerId ?? "anthropic"}
              />
            </ConfigField>
            <ConfigField label={t("chat.config.model")} onReset={() => resetOverride("modelId")} source={sourceOf("modelId")}>
              <ModelSelect
                disabled={lockRuntimeIdentity}
                models={availableModels}
                onChange={(value) => setOverride("modelId", value)}
                onClose={closeField}
                onOpen={() => toggleField("model")}
                open={openField === "model"}
                value={effectiveConfig.modelId ?? availableModels[0]?.id ?? ""}
              />
            </ConfigField>
          </section>

          <section className="grid gap-1.5">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.reasoning")}</h3>
            {availableReasoning.length ? (
              <ConfigField label={t("chat.config.reasoningLabel")} onReset={() => resetOverride("reasoningDepth")} source={sourceOf("reasoningDepth")}>
                <ReasoningSelect
                  availableReasoning={availableReasoning}
                  onChange={(value) => setOverride("reasoningDepth", value)}
                  onClose={closeField}
                  onOpen={() => toggleField("reasoning")}
                  open={openField === "reasoning"}
                  value={effectiveConfig.reasoningDepth ?? "low"}
                />
              </ConfigField>
            ) : null}
            <ConfigField label={t("chat.config.thinking")} onReset={() => resetOverride("thinking")} source={sourceOf("thinking")}>
              <ConfigToggle checked={effectiveConfig.thinking} label={t("chat.config.thinking")} onChange={(value) => setOverride("thinking", value)} />
            </ConfigField>
            <ConfigField label={t("chat.config.streaming")} onReset={() => resetOverride("streaming")} source={sourceOf("streaming")}>
              <ConfigToggle checked={effectiveConfig.streaming} label={t("chat.config.streaming")} onChange={(value) => setOverride("streaming", value)} />
            </ConfigField>
          </section>

          <section className="grid gap-1.5">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.permission")}</h3>
            {effectiveConfig.agentPolicy && displayedPolicy ? (
              <p className={cn("text-xs", highRisk ? "font-medium text-warning" : "text-muted-foreground")} data-testid="composer-popover-effective-policy">
                {t("chat.config.execution.effective", {
                  effective: t(`chat.config.execution.effective.${displayedPolicy}`),
                  policy: t(`settings.agentPolicies.template.${effectiveConfig.agentPolicy}`),
                })}
              </p>
            ) : null}
            <ConfigField label={t("chat.config.modeLabel")} onReset={() => resetOverride("executionMode")} source={sourceOf("executionMode")}>
              <ModeSelect
                availableModes={availableModes}
                emphasizeCapabilities={effectiveConfig.agentId === "onepiece" || effectiveConfig.providerId === "onepiece"}
                onChange={(value) => setOverride("executionMode", value)}
                onClose={closeField}
                onOpen={() => toggleField("mode")}
                open={openField === "mode"}
                value={effectiveConfig.executionMode}
              />
            </ConfigField>
          </section>

          <section className="grid gap-1">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.override")}</h3>
            <p className="text-xs text-muted-foreground">{hasOverrides ? t("chat.config.resetAllHint") : t("chat.config.noOverrides")}</p>
          </section>

          <section className="grid gap-1.5">
            <h3 className="text-xs font-semibold text-foreground">{t("chat.config.group.advanced")}</h3>
            <ConfigField label={t("chat.config.longContext")} onReset={() => resetOverride("longContext")} source={sourceOf("longContext")}>
              <ConfigToggle checked={effectiveConfig.longContext} label={t("chat.config.longContext")} onChange={(value) => setOverride("longContext", value)} />
            </ConfigField>
          </section>
        </div>
      ) : null}
    </div>
  );
}
