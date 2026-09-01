import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChatInputBox } from "../components/chat/ChatInputBox";
import { ComposerMediaActions } from "../components/chat/ComposerMediaActions";
import { LocalMediaResultDialog } from "../components/chat/LocalMediaResultDialog";
import { OcrReviewDialog } from "../components/chat/OcrReviewDialog";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";
import { createChatOperationFailureEvent } from "../main-layout/chat-operation-failure";
import { useNotifications } from "../notifications/notification-provider";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { agentService } from "../services/runtime-agent-client";
import { settingsService } from "../services/runtime-settings-client";
import { activeSeatsFromSession } from "../services/session-seats";
import { seatMentionOptions } from "../services/seat-mention-options";
import { canSendToSession } from "../services/session-admission";
import { useSlashCommands } from "../services/slash-commands/use-slash-commands";
import type { SlashCommandNavigation } from "../services/slash-commands/types";
import { useLocalMediaComposer } from "./local-media/use-local-media-composer";
import { RunnerSelector } from "./runner-selector";
import { useRunnerSelection } from "./use-runner-selection";

const NO_NAVIGATION: SlashCommandNavigation = {
  openDestination: () => undefined, openSessionTab: () => undefined,
};

export function ApiSessionComposer({
  model,
  navigation,
}: {
  model: MainLayoutModel;
  navigation?: SlashCommandNavigation;
}) {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const isMultiSeat = Boolean(model.activeSession && activeSeatsFromSession(model.activeSession).length > 1);
  const roles = useSessionRoles(isMultiSeat);
  const participantMentions = seatMentionOptions(model.activeSession, model.agents, roles);
  const [pendingLiteralSend, setPendingLiteralSend] = useState(false);
  const runner = useRunnerSelection(model.activeSession, model.agents);
  const slash = useSlashCommands({
    session: model.activeSession,
    config: model.chatConfig.config,
    isStreaming: model.isStreaming,
    chat: {
      setSessionExecutionMode: model.chatConfig.setSessionExecutionMode,
      setStreaming: model.chatConfig.setStreaming,
      setThinking: model.chatConfig.setThinking,
      setLongContext: model.chatConfig.setLongContext,
    },
    actions: {
      exportSession: model.exportSession,
      loadUsageSummary: async (sessionId) => {
        const summary = await agentService.getSessionUsageSummary(sessionId);
        return {
          totalTokens: summary.reported.totalTokens,
          inputTokens: summary.reported.inputTokens,
          outputTokens: summary.reported.outputTokens,
          responseCount: summary.responseCount,
        };
      },
    },
    navigate: navigation ?? NO_NAVIGATION,
    // Same channel `use-main-layout-model` reports chat failures through, so a failure a command
    // absorbed to show its own message still reaches the unified log rather than only the screen.
    onError: (source, reason) => {
      const event = createChatOperationFailureEvent(source, reason);
      notify({ type: "error", title: t("app.error.title"), message: event.message, scope: { kind: "global" } });
      void settingsService.reportClientLogEvent(event).catch(() => undefined);
    },
  });

  // The draft is read through a ref rather than captured, because a media result can arrive
  // seconds after the action started and must join whatever the user has typed since.
  const draftRef = useRef(model.draft);
  draftRef.current = model.draft;
  const selectionRef = useRef<{ start: number; end: number } | null>(null);
  const textAreaRef = useRef<HTMLTextAreaElement | null>(null);
  const setDraftFromMedia = useCallback(
    (value: string) => {
      // The same setter the textarea uses, so slash and file-reference suggestions stay in step.
      slash.updateSuggestions(value);
      model.setDraft(value);
    },
    [model, slash],
  );
  const media = useLocalMediaComposer({
    composerScopeId: model.activeSession?.id ?? null,
    getDraft: () => draftRef.current,
    setDraft: setDraftFromMedia,
    getSelection: () => selectionRef.current,
    getTextArea: () => textAreaRef.current,
  });

  // `model.submit()` reads `model.draft`, so an unescaped literal has to land in state before the
  // send happens — one render apart, not one statement apart.
  useEffect(() => {
    if (!pendingLiteralSend) return;
    setPendingLiteralSend(false);
    model.submitWithRunner(runner.selection);
  }, [model, pendingLiteralSend, runner.selection]);

  function submit() {
    const outcome = slash.dispatch(model.draft);
    if (outcome.kind === "handled") { model.setDraft(""); return; }
    if (outcome.kind === "literal") { model.setDraft(outcome.content); setPendingLiteralSend(true); return; }
    model.submitWithRunner(runner.selection);
  }

  const runnerSelector = (
    <RunnerSelector
      descriptors={runner.descriptors}
      disabled={!canSendToSession(model.activeSession) || model.isSending || model.isStreaming}
      error={runner.error}
      loading={runner.loading}
      onChange={runner.setSelection}
      onRetry={() => void runner.refetch()}
      value={runner.selection}
    />
  );

  return (
    <div>
      <ChatInputBox
      agents={model.chatConfig.availableAgents}
      availableModes={model.chatConfig.availableModes}
      availableModels={model.chatConfig.availableModels}
      availableReasoning={model.chatConfig.availableReasoning}
      disabled={!canSendToSession(model.activeSession) || model.isSending}
      fileReferenceCandidates={model.fileReferenceCandidates}
      fileReferences={model.fileReferences}
      isStreaming={model.isStreaming}
      lockRuntimeIdentity
      mediaActions={<ComposerMediaActions hasText={model.draft.trim().length > 0} media={media} />}
      participantMentions={participantMentions}
      runConfig={model.runConfigOverrides}
      runnerSelector={runnerSelector}
      slashCommandOutput={slash.output}
      slashCommandSuggestions={slash.suggestions}
      onAddFileReference={model.addFileReference}
      onChange={(value) => { slash.updateSuggestions(value); model.setDraft(value); }}
      onClear={() => model.setDraft("")}
      onDismissSlashCommandOutput={slash.dismissOutput}
      onRemoveFileReference={model.removeFileReference}
      onSelectionChange={(range) => { selectionRef.current = range; }}
      onSelectSlashCommand={(name) => model.setDraft(slash.completeDraft(name))}
      onStop={model.stop}
      onSubmit={submit}
      sessionId={model.activeSession?.id ?? null}
      textAreaRef={textAreaRef}
      value={model.draft}
      />
      {media.review ? (
        <OcrReviewDialog
          onAppend={media.appendReviewText}
          onCancel={media.cancelReview}
          onChange={media.updateReviewText}
          review={media.review}
        />
      ) : null}
      {media.overflow ? (
        <LocalMediaResultDialog
          engine={media.overflow.engine}
          onClose={media.dismissOverflow}
          text={media.overflow.text}
        />
      ) : null}
    </div>
  );
}
