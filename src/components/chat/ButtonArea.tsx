import { useState, type ReactNode } from "react";
import { Send, Sparkles, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig, ModelInfo, SessionExecutionMode, ReasoningDepth } from "../../types/chat";
import { Button } from "../ui/button";
import { ConfigSelect, ModelSelect, ModeSelect, ProviderSelect, ReasoningSelect } from "./selectors";

type OpenDropdown = "config" | "provider" | "mode" | "model" | "reasoning" | null;

export function ButtonArea({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  canSubmit,
  config,
  disabled,
  isStreaming,
  lockRuntimeIdentity = false,
  mediaActions,
  onAgentChange,
  onEnhance,
  onLongContextChange,
  onModeChange,
  onModelChange,
  onProviderChange,
  onReasoningChange,
  onStop,
  onStreamingChange,
  onSubmit,
  onThinkingChange,
}: {
  agents: AgentRegistryEntry[];
  availableModes: SessionExecutionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  canSubmit: boolean;
  config: ChatConfig;
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
  onAgentChange: (value: string) => void;
  onEnhance?: () => void;
  onLongContextChange: (value: boolean) => void;
  onModeChange: (value: SessionExecutionMode) => void;
  onModelChange: (value: string) => void;
  onProviderChange: (value: string) => void;
  onReasoningChange: (value: ReasoningDepth) => void;
  onStop: () => void;
  onStreamingChange: (value: boolean) => void;
  onSubmit: () => void;
  onThinkingChange: (value: boolean) => void;
}) {
  const { t } = useTranslation();
  const [openDropdown, setOpenDropdown] = useState<OpenDropdown>(null);
  const open = (id: OpenDropdown) => setOpenDropdown((current) => (current === id ? null : id));
  const close = () => setOpenDropdown(null);

  return (
    <div className="flex min-h-11 flex-wrap items-center gap-2 px-2 pb-2 pt-1" data-testid="composer-toolbar">
      <div className="flex min-w-0 flex-wrap items-center gap-1.5">
        <ConfigSelect
          agents={agents}
          longContext={config.longContext}
          onAgentChange={onAgentChange}
          onClose={close}
          onLongContextChange={onLongContextChange}
          onOpen={() => open("config")}
          onStreamingChange={onStreamingChange}
          onThinkingChange={onThinkingChange}
          open={openDropdown === "config"}
          selectedAgentId={config.agentId}
          streaming={config.streaming}
          thinking={config.thinking}
        />
        <ProviderSelect
          disabled={lockRuntimeIdentity}
          onChange={onProviderChange}
          onClose={close}
          onOpen={() => open("provider")}
          open={openDropdown === "provider"}
          value={config.providerId ?? "anthropic"}
        />
        <ModeSelect
          availableModes={availableModes}
          emphasizeCapabilities={config.agentId === "onepiece" || config.providerId === "onepiece"}
          onChange={onModeChange}
          onClose={close}
          onOpen={() => open("mode")}
          open={openDropdown === "mode"}
          value={config.executionMode}
        />
        {config.agentPolicy && config.effectiveExecutionPolicy ? (
          <span className="max-w-64 truncate text-[11px] text-muted-foreground" data-testid="effective-execution-policy">
            {t("chat.config.execution.effective", {
              effective: t(`chat.config.execution.effective.${config.effectiveExecutionPolicy}`),
              policy: t(`settings.agentPolicies.template.${config.agentPolicy}`),
            })}
          </span>
        ) : null}
        <ModelSelect
          disabled={lockRuntimeIdentity}
          models={availableModels}
          onChange={onModelChange}
          onClose={close}
          onOpen={() => open("model")}
          open={openDropdown === "model"}
          value={config.modelId ?? availableModels[0]?.id ?? ""}
        />
        <ReasoningSelect
          availableReasoning={availableReasoning}
          onChange={onReasoningChange}
          onClose={close}
          onOpen={() => open("reasoning")}
          open={openDropdown === "reasoning"}
          value={config.reasoningDepth ?? "low"}
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
