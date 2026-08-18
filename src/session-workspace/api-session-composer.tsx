import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChatInputBox } from "../components/chat/ChatInputBox";
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
import { RunnerSelector } from "./runner-selector";
import { useRunnerSelection } from "./use-runner-selection";

const NO_NAVIGATION: SlashCommandNavigation = {
  openAssociatedPlan: null, openDestination: () => undefined, openSessionTab: () => undefined,
};

export function ApiSessionComposer({
  model,
  navigation,
  onOpenPlan,
}: {
  model: MainLayoutModel;
  navigation?: SlashCommandNavigation;
  onOpenPlan?: () => void;
}) {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const isMultiSeat = Boolean(model.activeSession && activeSeatsFromSession(model.activeSession).length > 1);
  const roles = useSessionRoles(isMultiSeat);
  const participantMentions = seatMentionOptions(model.activeSession, model.agents, roles);
  const [pendingLiteralSend, setPendingLiteralSend] = useState(false);
  const runner = useRunnerSelection(model.activeSession, model.agents);
  // A session with no plan run has nothing for `/plan` to open, and offering a command that does
  // nothing is worse than not offering it — so the handler is withheld, not just left unused.
  const openPlan = model.chatConfig.associatedPlanRun ? onOpenPlan : undefined;

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
    navigate: navigation ?? { ...NO_NAVIGATION, openAssociatedPlan: openPlan ?? null },
    // Same channel `use-main-layout-model` reports chat failures through, so a failure a command
    // absorbed to show its own message still reaches the unified log rather than only the screen.
    onError: (source, reason) => {
      const event = createChatOperationFailureEvent(source, reason);
      notify({ type: "error", title: t("app.error.title"), message: event.message, scope: { kind: "global" } });
      void settingsService.reportClientLogEvent(event).catch(() => undefined);
    },
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

  return (
    <div>
      <RunnerSelector
        descriptors={runner.descriptors}
        disabled={!canSendToSession(model.activeSession) || model.isSending || model.isStreaming}
        error={runner.error}
        loading={runner.loading}
        onChange={runner.setSelection}
        onRetry={() => void runner.refetch()}
        value={runner.selection}
      />
      <ChatInputBox
      agents={model.chatConfig.availableAgents}
      availableModes={model.chatConfig.availableModes}
      availableModels={model.chatConfig.availableModels}
      availableReasoning={model.chatConfig.availableReasoning}
      config={model.chatConfig.config}
      disabled={!canSendToSession(model.activeSession) || model.isSending}
      fileReferenceCandidates={model.fileReferenceCandidates}
      fileReferences={model.fileReferences}
      isStreaming={model.isStreaming}
      lockRuntimeIdentity
      participantMentions={participantMentions}
      slashCommandOutput={slash.output}
      slashCommandSuggestions={slash.suggestions}
      onAddFileReference={model.addFileReference}
      onChange={(value) => { slash.updateSuggestions(value); model.setDraft(value); }}
      onClear={() => model.setDraft("")}
      onConfigAgentChange={model.chatConfig.changeAgent}
      onConfigLongContextChange={model.chatConfig.setLongContext}
      onConfigModeChange={model.chatConfig.setSessionExecutionMode}
      onConfigModelChange={model.chatConfig.changeModel}
      onConfigProviderChange={model.chatConfig.changeProvider}
      onConfigReasoningChange={model.chatConfig.setReasoningDepth}
      onConfigStreamingChange={model.chatConfig.setStreaming}
      onConfigThinkingChange={model.chatConfig.setThinking}
      onDismissSlashCommandOutput={slash.dismissOutput}
      onOpenPlan={openPlan}
      onRemoveFileReference={model.removeFileReference}
      onSelectSlashCommand={(name) => model.setDraft(slash.completeDraft(name))}
      onStop={model.stop}
      onSubmit={submit}
      sessionId={model.activeSession?.id ?? null}
      value={model.draft}
      />
    </div>
  );
}
