import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig, ModelInfo, PermissionMode, ReasoningDepth } from "../../types/chat";
import { ButtonArea } from "./ButtonArea";

export function ChatInputBox({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  config,
  disabled,
  isStreaming,
  onChange,
  onClear,
  onConfigAgentChange,
  onConfigLongContextChange,
  onConfigModeChange,
  onConfigModelChange,
  onConfigProviderChange,
  onConfigReasoningChange,
  onConfigStreamingChange,
  onConfigThinkingChange,
  onStop,
  onSubmit,
  value,
}: {
  agents: AgentRegistryEntry[];
  availableModes: PermissionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  config: ChatConfig;
  disabled?: boolean;
  isStreaming: boolean;
  onChange: (value: string) => void;
  onClear: () => void;
  onConfigAgentChange: (value: string) => void;
  onConfigLongContextChange: (value: boolean) => void;
  onConfigModeChange: (value: PermissionMode) => void;
  onConfigModelChange: (value: string) => void;
  onConfigProviderChange: (value: string) => void;
  onConfigReasoningChange: (value: ReasoningDepth) => void;
  onConfigStreamingChange: (value: boolean) => void;
  onConfigThinkingChange: (value: boolean) => void;
  onStop: () => void;
  onSubmit: () => void;
  value: string;
}) {
  const { t } = useTranslation();
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const canSubmit = value.trim().length > 0 && !disabled && !isStreaming;

  useEffect(() => {
    const element = textAreaRef.current;
    if (!element) return;
    element.style.height = "40px";
    element.style.height = `${Math.min(200, Math.max(40, element.scrollHeight))}px`;
    element.style.overflowY = element.scrollHeight > 200 ? "auto" : "hidden";
  }, [value]);

  return (
    <div className="shrink-0 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="relative">
        <textarea
          className="ucd-input min-h-10 w-full resize-none rounded-md px-3 py-2 pr-10 text-sm leading-6 outline-none focus-visible:ring-2 focus-visible:ring-ring"
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
            event.preventDefault();
            if (canSubmit) onSubmit();
          }}
          placeholder={disabled ? t("chat.placeholderDisabled") : t("chat.placeholder")}
          ref={textAreaRef}
          value={value}
        />
        {value ? (
          <button
            className="absolute right-2 top-2 flex h-7 w-7 items-center justify-center rounded text-muted-foreground hover:bg-muted"
            disabled={disabled || isStreaming}
            onClick={onClear}
            title={t("chat.clear")}
            type="button"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
        ) : null}
      </div>
      <ButtonArea
        agents={agents}
        availableModes={availableModes}
        availableModels={availableModels}
        availableReasoning={availableReasoning}
        canSubmit={canSubmit}
        config={config}
        disabled={disabled}
        isStreaming={isStreaming}
        onAgentChange={onConfigAgentChange}
        onLongContextChange={onConfigLongContextChange}
        onModeChange={onConfigModeChange}
        onModelChange={onConfigModelChange}
        onProviderChange={onConfigProviderChange}
        onReasoningChange={onConfigReasoningChange}
        onStop={onStop}
        onStreamingChange={onConfigStreamingChange}
        onSubmit={onSubmit}
        onThinkingChange={onConfigThinkingChange}
      />
    </div>
  );
}
