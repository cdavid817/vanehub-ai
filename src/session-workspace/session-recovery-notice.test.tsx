// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import type { SessionRecoverySummary } from "../services/agent-service";
import { renderWithAppProviders } from "../test/render";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { SessionRecoveryNotice } from "./session-recovery-notice";
import { SessionWorkspaceRegionsHost } from "./session-tabs";

vi.mock("../components/chat/MessageList", () => ({
  MessageList: ({ messages }: { messages: ChatMessage[] }) => (
    <div>{messages.map((message) => message.content).join("|")}</div>
  ),
}));

function session(recoveryStatus: Session["recoveryStatus"]): Session {
  return {
    id: "session-1",
    title: "Recovered session",
    agentId: "onepiece",
    interactionMode: "api",
    personalizationMode: "standard", lifecycleState: "failed",
    recoveryStatus,
    recoveryRevision: 1,
    stateRevision: 2,
    historyRevision: 3,
    activeExecutionRunId: null,
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-09T00:00:00Z",
    updatedAt: "2026-08-09T00:00:00Z",
  };
}

function summary(value: Session): SessionRecoverySummary {
  return {
    session: value,
    latestReport: {
      reportId: "report-1",
      sessionId: value.id,
      recoveryRevision: value.recoveryRevision,
      trigger: "startup",
      observedLifecycle: "running",
      observedExecutionRunId: "run-1",
      decision: value.recoveryStatus === "quarantined" ? "quarantined" : "action_required",
      reasonCodes: ["unfinished_tool_activity"],
      evidenceRefs: [{
        kind: "session",
        sessionId: value.id,
        stateRevision: value.stateRevision,
        historyRevision: value.historyRevision,
      }],
      createdAt: "2026-08-09T00:00:00Z",
    },
  };
}

const partialMessage = {
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "Partial response remains readable",
  status: "failed",
  createdAt: "2026-08-09T00:00:00Z",
  updatedAt: "2026-08-09T00:00:00Z",
  sessionSequence: 1,
  executionRunId: "run-1",
} satisfies ChatMessage;

describe("SessionRecoveryNotice", () => {
  it("preserves partial transcript content and confirms acknowledgement semantics", async () => {
    const activeSession = session("action_required");
    const onAcknowledge = vi.fn().mockResolvedValue(undefined);
    const notice = (
      <SessionRecoveryNotice
        acknowledging={false}
        onAcknowledge={onAcknowledge}
        session={activeSession}
        summary={summary(activeSession)}
      />
    );
    const { user } = renderWithAppProviders(
      <SessionWorkspaceRegionsHost
        activeSession={activeSession}
        apiComposer={<div>Composer remains mounted</div>}
        messages={[partialMessage]}
        messagesPartial={false}
        onOpenSettings={vi.fn()}
        recoveryNotice={notice}
        sessionActivationKey={0}
      />,
    );

    expect(screen.getByText(partialMessage.content)).toBeTruthy();
    expect(screen.getByTestId("session-recovery-notice").getAttribute("role")).toBe("alert");
    await user.click(screen.getByRole("button", { name: i18n.t("recovery.acknowledge.open") }));
    expect(screen.getByText(i18n.t("recovery.acknowledge.noRetry"))).toBeTruthy();
    expect(screen.getByText(i18n.t("recovery.acknowledge.uncertainEffect"))).toBeTruthy();
    await user.click(screen.getByRole("button", { name: i18n.t("recovery.acknowledge.confirm") }));
    expect(onAcknowledge).toHaveBeenCalledOnce();
  });

  it.each(["reconciling", "quarantined"] as const)("presents %s as non-actionable recovery", (status) => {
    const activeSession = session(status);
    renderWithAppProviders(
      <SessionRecoveryNotice
        acknowledging={false}
        onAcknowledge={vi.fn()}
        session={activeSession}
        summary={summary(activeSession)}
      />,
    );

    expect(screen.getByText(i18n.t(`recovery.${status}.title`))).toBeTruthy();
    expect(screen.queryByRole("button", { name: i18n.t("recovery.acknowledge.open") })).toBeNull();
  });
});
