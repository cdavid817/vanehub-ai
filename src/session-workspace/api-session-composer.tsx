import { ChatInputBox } from "../components/chat/ChatInputBox";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { activeSeatsFromSession } from "../services/session-seats";
import { seatMentionOptions } from "../services/seat-mention-options";
import { canSendToSession } from "../services/session-admission";

export function ApiSessionComposer({ model, onOpenPlan }: { model: MainLayoutModel; onOpenPlan?: () => void }) {
  const isMultiSeat = Boolean(model.activeSession && activeSeatsFromSession(model.activeSession).length > 1);
  const roles = useSessionRoles(isMultiSeat);
  const participantMentions = seatMentionOptions(model.activeSession, model.agents, roles);
  return (
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
      onAddFileReference={model.addFileReference}
      onChange={model.setDraft}
      onClear={() => model.setDraft("")}
      onConfigAgentChange={model.chatConfig.changeAgent}
      onConfigLongContextChange={model.chatConfig.setLongContext}
      onConfigModeChange={model.chatConfig.setSessionExecutionMode}
      onConfigModelChange={model.chatConfig.changeModel}
      onConfigProviderChange={model.chatConfig.changeProvider}
      onConfigReasoningChange={model.chatConfig.setReasoningDepth}
      onConfigStreamingChange={model.chatConfig.setStreaming}
      onConfigThinkingChange={model.chatConfig.setThinking}
      onOpenPlan={model.chatConfig.associatedPlanRun ? onOpenPlan : undefined}
      onRemoveFileReference={model.removeFileReference}
      onStop={model.stop}
      onSubmit={model.submit}
      value={model.draft}
    />
  );
}
