import { useEffect, useRef, type ReactNode, type RefObject } from "react";
import { FileText, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useComposerDropTarget } from "../../hooks/use-composer-drop-target";
import { useComposerMention } from "../../hooks/use-composer-mention";
import { useComposerCompletionNavigation } from "../../hooks/use-composer-completion-navigation";
import { cn } from "../../lib/utils";
import { LazyFeature } from "../lazy-feature";

// The preview pulls in highlight.js, which is far too heavy to sit in the main bundle for
// a dialog most messages never open. It loads on first use instead.
const loadPreviewDialog = () =>
  import("./FileReferencePreviewDialog").then((module) => ({ default: module.FileReferencePreviewDialog }));
import { formatMentionRange, type MentionLineRange } from "../../services/composer-mention";
import type { AgentRegistryEntry } from "../../types/agent";
import type { FileSearchMatch } from "../../types/session-workspace";
import type { ChatConfig, ChatFileReference, ModelInfo, ReasoningDepth, SessionExecutionMode } from "../../types/chat";
import { ButtonArea } from "./ButtonArea";
import { ComposerMentionCompletion } from "./ComposerMentionCompletion";
import { FileReferenceLines } from "./FileReferenceLines";
import type { SeatMentionOption } from "./SeatMentionCompletion";
import { SlashCommandCompletion } from "./SlashCommandCompletion";
import { SlashCommandOutput } from "./SlashCommandOutput";
import type { CommandOutput, SlashCommand } from "../../services/slash-commands/types";

export function ChatInputBox({
  agents,
  availableModes,
  availableModels,
  availableReasoning,
  config,
  disabled,
  isStreaming,
  lockRuntimeIdentity = false,
  fileReferenceCandidates,
  mediaActions,
  onSelectionChange,
  textAreaRef: externalTextAreaRef,
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
  participantMentions = [],
  sessionId = null,
  slashCommandOutput = null,
  slashCommandSuggestions = [],
  onDismissSlashCommandOutput,
  onSelectSlashCommand,
  value,
}: {
  agents: AgentRegistryEntry[];
  availableModes: SessionExecutionMode[];
  availableModels: ModelInfo[];
  availableReasoning: ReasoningDepth[];
  config: ChatConfig;
  disabled?: boolean;
  isStreaming: boolean;
  lockRuntimeIdentity?: boolean;
  fileReferenceCandidates: FileSearchMatch[];
  /** Local-media action group, rendered in the toolbar's right cluster. */
  mediaActions?: ReactNode;
  /** Reports the textarea selection so read-aloud can speak exactly what is highlighted. */
  onSelectionChange?: (range: { start: number; end: number } | null) => void;
  /**
   * Published so the local-media controller can return the caret after it appends text.
   *
   * A callback would be the smaller surface, but restoring focus needs the element itself: the
   * caret has to land at the end of the new draft, which is a `setSelectionRange` on the node.
   */
  textAreaRef?: RefObject<HTMLTextAreaElement | null>;
  fileReferences: ChatFileReference[];
  onChange: (value: string) => void;
  onAddFileReference: (candidate: FileSearchMatch, range: MentionLineRange) => void;
  onClear: () => void;
  onConfigAgentChange: (value: string) => void;
  onConfigLongContextChange: (value: boolean) => void;
  onConfigModeChange: (value: SessionExecutionMode) => void;
  onConfigModelChange: (value: string) => void;
  onConfigProviderChange: (value: string) => void;
  onConfigReasoningChange: (value: ReasoningDepth) => void;
  onConfigStreamingChange: (value: boolean) => void;
  onConfigThinkingChange: (value: boolean) => void;
  onStop: () => void;
  onSubmit: () => void;
  onRemoveFileReference: (referenceId: string) => void;
  participantMentions?: SeatMentionOption[];
  /** Needed to read a candidate's content for the preview; without one, selection attaches directly. */
  sessionId?: string | null;
  slashCommandOutput?: CommandOutput | null;
  slashCommandSuggestions?: SlashCommand[];
  onDismissSlashCommandOutput?: () => void;
  onSelectSlashCommand?: (name: string) => void;
  value: string;
}) {
  const { t } = useTranslation();
  const localTextAreaRef = useRef<HTMLTextAreaElement>(null);
  const textAreaRef = externalTextAreaRef ?? localTextAreaRef;
  const canSubmit = value.trim().length > 0 && !disabled && !isStreaming;
  const { applyMention, fileSuggestions, mentionRange, participantSuggestions, pendingPreview, setPendingPreview } = useComposerMention({
    disabled,
    fileReferenceCandidates,
    fileReferences,
    participantMentions,
    value,
  });
  const completion = useComposerCompletionNavigation([
    ...participantSuggestions.map((option) => `participant:${option.mention}`),
    ...fileSuggestions.map((candidate) => `file:${candidate.path}`),
  ]);

  const dropTarget = useComposerDropTarget({
    disabled: Boolean(disabled) || isStreaming,
    // A dropped or pasted path names a file, not a region, so it attaches whole.
    onAttachPath: (path) => onAddFileReference({ name: path.split("/").pop() ?? path, path }, {}),
  });

  function attachReference(candidate: FileSearchMatch, range: MentionLineRange) {
    onAddFileReference(candidate, range);
    onChange(applyMention(`${candidate.path}${formatMentionRange(range)}`));
    textAreaRef.current?.focus();
  }

  function selectReference(candidate: FileSearchMatch) {
    // A range typed by hand already says which lines are meant, so previewing would only
    // ask the user to say it twice.
    if (mentionRange.startLine !== undefined) {
      attachReference(candidate, mentionRange);
      return;
    }
    if (sessionId) setPendingPreview({ candidate, sessionId });
    else attachReference(candidate, {});
  }

  function selectParticipant(mentionHandle: string) {
    onChange(applyMention(mentionHandle));
    textAreaRef.current?.focus();
  }

  function activateCompletion(index: number) {
    const participant = participantSuggestions[index];
    if (participant) selectParticipant(participant.mention);
    else {
      const candidate = fileSuggestions[index - participantSuggestions.length];
      if (candidate) selectReference(candidate);
    }
  }

  // A pending preview belongs to the session it was opened from. Left alone across a
  // session switch it would keep the dialog open and re-read the old path against the new
  // session — which silently shows a different file whenever both sessions happen to
  // contain that path.
  useEffect(() => {
    setPendingPreview(null);
  }, [sessionId, setPendingPreview]);

  useEffect(() => {
    const element = textAreaRef.current;
    if (!element) return;
    element.style.height = "88px";
    element.style.height = `${Math.min(200, Math.max(88, element.scrollHeight))}px`;
    element.style.overflowY = element.scrollHeight > 200 ? "auto" : "hidden";
  }, [textAreaRef, value]);

  return (
    <div className="shrink-0 bg-transparent px-3 py-3">
      {pendingPreview && pendingPreview.sessionId === sessionId ? (
        <LazyFeature
          componentProps={{
            name: pendingPreview.candidate.name,
            onAttach: (range: MentionLineRange) => {
              attachReference(pendingPreview.candidate, range);
              setPendingPreview(null);
            },
            onCancel: () => {
              setPendingPreview(null);
              textAreaRef.current?.focus();
            },
            path: pendingPreview.candidate.path,
            sessionId: pendingPreview.sessionId,
          }}
          loader={loadPreviewDialog}
        />
      ) : null}
      <div
        className={cn(
          "relative bg-background transition-colors focus-within:bg-muted/10 focus-within:ring-1 focus-within:ring-inset focus-within:ring-ring/30",
          dropTarget.isDropTarget && "bg-primary/[0.04] ring-1 ring-inset ring-ring/40",
        )}
        data-drop-target={dropTarget.isDropTarget ? "true" : undefined}
        data-testid="wechat-style-composer"
        {...dropTarget.handlers}
      >
        {/* Single positioned wrapper so the output panel and whichever completion panel is
            active stack in flow order instead of layering at identical coordinates. The
            drop hint joins them for the same reason — and because the composer's top edge,
            where it used to sit, is where attached reference chips live. */}
        {dropTarget.isDropTarget || slashCommandOutput || slashCommandSuggestions.length || participantSuggestions.length || fileSuggestions.length ? (
          <div className="absolute bottom-full left-0 z-20 mb-2 flex w-full flex-col gap-2">
            {dropTarget.isDropTarget ? (
              <p className="pointer-events-none w-full rounded-md bg-primary/10 px-3 py-1.5 text-center text-xs font-medium text-primary" role="status">
                {t("chat.dropFileHint")}
              </p>
            ) : null}
            <SlashCommandOutput onDismiss={onDismissSlashCommandOutput ?? (() => undefined)} output={slashCommandOutput} />
            {slashCommandSuggestions.length ? (
              <div className="ucd-panel grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-1 text-xs shadow-lg">
                <SlashCommandCompletion onSelect={onSelectSlashCommand ?? (() => undefined)} options={slashCommandSuggestions} />
              </div>
            ) : null}
            <ComposerMentionCompletion activeIndex={completion.activeIndex} fileSuggestions={fileSuggestions} listboxId={completion.listboxId} onSelectFile={selectReference} onSelectParticipant={selectParticipant} optionId={completion.optionId} participantSuggestions={participantSuggestions} />
          </div>
        ) : null}
        {fileReferences.length ? (
          <div className="flex flex-wrap gap-1.5 px-3 pt-3" data-testid="composer-file-references">
            {fileReferences.map((reference) => (
              <span className="inline-flex max-w-full items-center gap-1 rounded-md border border-border bg-muted/60 px-2 py-1 text-xs" key={reference.id}>
                <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
                <span className="truncate">{reference.name}</span>
                <FileReferenceLines reference={reference} />
                <button className="rounded text-muted-foreground hover:text-foreground" disabled={disabled || isStreaming} onClick={() => onRemoveFileReference(reference.id)} title={t("chat.removeFileReference")} type="button">
                  <X className="h-3 w-3" aria-hidden="true" />
                </button>
              </span>
            ))}
          </div>
        ) : null}
        <div className="relative">
        <textarea
          aria-activedescendant={completion.activeOptionId}
          aria-controls={participantSuggestions.length || fileSuggestions.length ? completion.listboxId : undefined}
          aria-expanded={participantSuggestions.length + fileSuggestions.length > 0}
          aria-haspopup="listbox"
          className="min-h-22 w-full resize-none border-0 bg-transparent px-3 py-3 pr-10 text-sm leading-6 outline-hidden placeholder:text-muted-foreground focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (completion.onKeyDown(event, activateCompletion)) return;
            if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
            event.preventDefault();
            if (canSubmit) onSubmit();
          }}
          onSelect={(event) =>
            onSelectionChange?.({
              start: event.currentTarget.selectionStart,
              end: event.currentTarget.selectionEnd,
            })
          }
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
          mediaActions={mediaActions}
          config={config}
          disabled={disabled}
          isStreaming={isStreaming}
          lockRuntimeIdentity={lockRuntimeIdentity}
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
    </div>
  );
}
