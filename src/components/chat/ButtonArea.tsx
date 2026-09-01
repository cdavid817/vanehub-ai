import type { ReactNode } from "react";
import { Send, Sparkles, Square, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ModelInfo, SessionExecutionMode, ReasoningDepth } from "../../types/chat";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ComposerConfigPopover } from "./ComposerConfigPopover";
import { effectivePolicyForDisplay, isHighRiskExecutionPolicy } from "./composer-risk";
import type { RunConfigurationOverrides } from "./hooks/useRunConfigurationOverrides";

export function ButtonArea({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  canSubmit,
  disabled,
  isStreaming,
  lockRuntimeIdentity = false,
  mediaActions,
  onEnhance,
  onStop,
  onSubmit,
  runConfig,
  runnerSelector,
}: {
  agents: AgentRegistryEntry[];
  availableModes: SessionExecutionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  canSubmit: boolean;
  disabled?: boolean;
  isStreaming: boolean;
  lockRuntimeIdentity?: boolean;
  /**
   * A narrow slot for the local-media action group.
   *
   * A slot rather than props because the alternative is this component learning about engine
   * readiness, recording state, and operation polling -- none of which is a toolbar concern, and
   * all of which would land here permanently once the first one did.
   */
  mediaActions?: ReactNode;
  onEnhance?: () => void;
  onStop: () => void;
  onSubmit: () => void;
  /** Effective config (profile + this-message overrides) and the provenance controls the
   *  popover needs (design.md Decision 9). Replaces the old `config` + eight
   *  `onConfigXChange` profile setters now that every field writes through `setOverride`. */
  runConfig: RunConfigurationOverrides;
  runnerSelector?: ReactNode;
}) {
  const { t } = useTranslation();
  const { effectiveConfig } = runConfig;
  const agentLabel = agents.find((agent) => agent.id === effectiveConfig.agentId)?.displayName ?? effectiveConfig.agentId;
  const resolvedModel = availableModels.find((model) => model.id === effectiveConfig.modelId) ?? availableModels[0];
  const modelLabel = resolvedModel?.label ?? t("chat.config.model");
  // Re-derived rather than read off `effectiveConfig.effectiveExecutionPolicy` directly: that
  // field is only recomputed by the server on a profile save, so it goes stale the moment
  // `executionMode` is staged as a this-message-only override (composer-risk.ts).
  const displayedPolicy = effectivePolicyForDisplay(effectiveConfig);
  // "allow" is the one effective-policy value the settings page already treats as risky enough
  // to need an explicit confirmation before assigning (composer-risk.ts) -- everything else
  // (readonly/ask) keeps the same plain, always-visible line it has today.
  const highRisk = isHighRiskExecutionPolicy(displayedPolicy);

  return (
    <div className="flex min-h-11 flex-wrap items-center gap-2 px-2 pb-2 pt-1" data-testid="composer-toolbar">
      <div className="flex min-w-0 flex-wrap items-center gap-1.5">
        <span className="min-w-0 truncate text-xs text-muted-foreground" data-testid="composer-config-summary">
          {t("chat.config.summary", { agent: agentLabel, model: modelLabel })}
        </span>
        {effectiveConfig.agentPolicy && displayedPolicy ? (
          <span className="inline-flex max-w-64 items-center gap-1 text-[11px]" data-testid="effective-execution-policy">
            {highRisk ? (
              <Badge className="shrink-0 gap-1" tone="warning">
                <TriangleAlert className="h-3 w-3" aria-hidden="true" />
                {t("chat.config.highRiskLabel")}
              </Badge>
            ) : null}
            <span className={cn("min-w-0 truncate", highRisk ? "font-medium text-warning" : "text-muted-foreground")}>
              {t("chat.config.execution.effective", {
                effective: t(`chat.config.execution.effective.${displayedPolicy}`),
                policy: t(`settings.agentPolicies.template.${effectiveConfig.agentPolicy}`),
              })}
            </span>
          </span>
        ) : null}
        <ComposerConfigPopover
          agents={agents}
          availableModes={availableModes}
          availableModels={availableModels}
          availableReasoning={availableReasoning}
          lockRuntimeIdentity={lockRuntimeIdentity}
          runConfig={runConfig}
          runnerSelector={runnerSelector}
        />
      </div>

      <div className="ml-auto flex items-center gap-2">
        {mediaActions}
        <Button className="h-8" disabled={disabled || !canSubmit || isStreaming} onClick={onEnhance} title={t("chat.enhanceTitle")} type="button" variant="ghost">
          <Sparkles className="h-4 w-4" aria-hidden="true" />
          {t("chat.enhance")}
        </Button>
        {isStreaming ? (
          <Button className="h-8" onClick={onStop} title={t("chat.stopTitle")} type="button" variant="outline">
            <Square className="h-4 w-4" aria-hidden="true" />
            {t("chat.stop")}
          </Button>
        ) : (
          <Button className="h-8 px-4" disabled={!canSubmit} onClick={onSubmit} title={t("chat.sendTitle")} type="button">
            <Send className="h-4 w-4" aria-hidden="true" />
            {t("chat.send")}
          </Button>
        )}
      </div>
    </div>
  );
}
