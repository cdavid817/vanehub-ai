import { useEffect, useMemo, useRef } from "react";
import { FileText, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../../types/agent";
import type { SessionDocument } from "../../types/session-workspace";
import type { ChatConfig, ChatFileReference, ModelInfo, PermissionMode, ReasoningDepth } from "../../types/chat";
import { ButtonArea } from "./ButtonArea";

export function ChatInputBox({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  config,
  disabled,
  isStreaming,
  fileReferenceCandidates,
  fileReferences,
  onAddFileReference,
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
  onRemoveFileReference,
  value,
}: {
  agents: AgentRegistryEntry[];
  availableModes: PermissionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  config: ChatConfig;
  disabled?: boolean;
  isStreaming: boolean;
  fileReferenceCandidates: SessionDocument[];
  fileReferences: ChatFileReference[];
  onChange: (value: string) => void;
  onAddFileReference: (document: SessionDocument) => void;
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
  onRemoveFileReference: (path: string) => void;
  value: string;
}) {
  const { t } = useTranslation();
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const canSubmit = value.trim().length > 0 && !disabled && !isStreaming;
  const mention = value.match(/(?:^|\s)@([^\s@]*)$/);
  const mentionQuery = mention?.[1]?.toLowerCase() ?? null;
  const suggestions = useMemo(() => {
    if (mentionQuery === null || disabled) return [];
    const selected = new Set(fileReferences.map((reference) => reference.path));
    return fileReferenceCandidates
      .filter((document) => !selected.has(document.path))
      .filter((document) => `${document.name} ${document.path}`.toLowerCase().includes(mentionQuery))
      .slice(0, 8);
  }, [disabled, fileReferenceCandidates, fileReferences, mentionQuery]);

  function selectReference(document: SessionDocument) {
    onAddFileReference(document);
    onChange(value.replace(/(?:^|\s)@([^\s@]*)$/, (token) => `${token.startsWith(" ") ? " " : ""}@${document.path} `));
    textAreaRef.current?.focus();
  }

  useEffect(() => {
    const element = textAreaRef.current;
    if (!element) return;
    element.style.height = "40px";
    element.style.height = `${Math.min(200, Math.max(40, element.scrollHeight))}px`;
    element.style.overflowY = element.scrollHeight > 200 ? "auto" : "hidden";
  }, [value]);

  return (
    <div className="shrink-0 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
      {fileReferences.length ? (
        <div className="mb-2 flex flex-wrap gap-1.5">
          {fileReferences.map((reference) => (
            <span className="inline-flex max-w-full items-center gap-1 rounded-md border border-border bg-background px-2 py-1 text-xs" key={reference.path}>
              <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
              <span className="truncate">{reference.name}</span>
              <button className="rounded text-muted-foreground hover:text-foreground" disabled={disabled || isStreaming} onClick={() => onRemoveFileReference(reference.path)} title={t("chat.removeFileReference")} type="button">
                <X className="h-3 w-3" aria-hidden="true" />
              </button>
            </span>
          ))}
        </div>
      ) : null}
      <div className="relative">
        {suggestions.length ? (
          <div className="ucd-panel absolute bottom-full left-0 z-20 mb-2 grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-1 text-xs shadow-lg">
            {suggestions.map((document) => (
              <button className="flex min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-muted" key={document.path} onClick={() => selectReference(document)} type="button">
                <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">{document.path}</span>
              </button>
            ))}
          </div>
        ) : null}
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
