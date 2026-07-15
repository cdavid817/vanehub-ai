import { useState } from "react";
import { Send, Sparkles, Square } from "lucide-react";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig, ModelInfo, PermissionMode, ReasoningDepth } from "../../types/chat";
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
  availableModes: PermissionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  canSubmit: boolean;
  config: ChatConfig;
  disabled?: boolean;
  isStreaming: boolean;
  onAgentChange: (value: string) => void;
  onEnhance?: () => void;
  onLongContextChange: (value: boolean) => void;
  onModeChange: (value: PermissionMode) => void;
  onModelChange: (value: string) => void;
  onProviderChange: (value: string) => void;
  onReasoningChange: (value: ReasoningDepth) => void;
  onStop: () => void;
  onStreamingChange: (value: boolean) => void;
  onSubmit: () => void;
  onThinkingChange: (value: boolean) => void;
}) {
  const [openDropdown, setOpenDropdown] = useState<OpenDropdown>(null);
  const open = (id: OpenDropdown) => setOpenDropdown((current) => (current === id ? null : id));
  const close = () => setOpenDropdown(null);

  return (
    <div className="mt-2 flex flex-wrap items-center gap-2">
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
          onChange={onProviderChange}
          onClose={close}
          onOpen={() => open("provider")}
          open={openDropdown === "provider"}
          value={config.providerId ?? "anthropic"}
        />
        <ModeSelect
          availableModes={availableModes}
          onChange={onModeChange}
          onClose={close}
          onOpen={() => open("mode")}
          open={openDropdown === "mode"}
          value={config.permissionMode}
        />
        <ModelSelect
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
        <Button disabled={disabled || !canSubmit || isStreaming} onClick={onEnhance} title="增强 Prompt" type="button" variant="outline">
          <Sparkles className="h-4 w-4" aria-hidden="true" />
          增强
        </Button>
        {isStreaming ? (
          <Button onClick={onStop} title="停止生成" type="button" variant="outline">
            <Square className="h-4 w-4" aria-hidden="true" />
            停止
          </Button>
        ) : (
          <Button disabled={!canSubmit} onClick={onSubmit} title="发送" type="button">
            <Send className="h-4 w-4" aria-hidden="true" />
            发送
          </Button>
        )}
      </div>
    </div>
  );
}
