import { ChatInputBox } from "../components/chat/ChatInputBox";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";

export function ApiSessionComposer({ model }: { model: MainLayoutModel }) {
  return (
    <ChatInputBox
      agents={model.chatConfig.availableAgents}
      availableModes={model.chatConfig.availableModes}
      availableModels={model.chatConfig.availableModels}
      availableReasoning={model.chatConfig.availableReasoning}
      config={model.chatConfig.config}
      disabled={!model.activeSession || model.activeSession.archived || model.isSending}
      fileReferenceCandidates={model.fileReferenceCandidates}
      fileReferences={model.fileReferences}
      isStreaming={model.isStreaming}
      lockRuntimeIdentity
      onAddFileReference={model.addFileReference}
      onChange={model.setDraft}
      onClear={() => model.setDraft("")}
      onConfigAgentChange={model.chatConfig.changeAgent}
      onConfigLongContextChange={model.chatConfig.setLongContext}
      onConfigModeChange={model.chatConfig.setPermissionMode}
      onConfigModelChange={model.chatConfig.changeModel}
      onConfigProviderChange={model.chatConfig.changeProvider}
      onConfigReasoningChange={model.chatConfig.setReasoningDepth}
      onConfigStreamingChange={model.chatConfig.setStreaming}
      onConfigThinkingChange={model.chatConfig.setThinking}
      onRemoveFileReference={model.removeFileReference}
      onStop={model.stop}
      onSubmit={model.submit}
      value={model.draft}
    />
  );
}
