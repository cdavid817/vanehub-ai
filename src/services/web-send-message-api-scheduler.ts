import { mockAgents } from "./mock-agent-data";
import { createWebMcpToolSimulationPlan } from "./web-mcp-tool-simulation";
import {
  createWebPendingApproval,
  isAgentAutoApproved,
} from "./web-permissions-mock-state";
import { createWebAgentMemory } from "./web-agent-memory-state";
import { publishWebChatEvent } from "./web-chat-state";
import {
  scheduleWebSendMessageTimeout,
  WEB_MOCK_PLAN_EXIT_TRIGGER,
  WEB_MOCK_QUESTION_TRIGGER,
  type WebSendMessageContext,
  type WebSendMessageTimeoutId,
} from "./web-send-message-context";

export function scheduleWebSendMessageApiTools(
  context: WebSendMessageContext,
  timeoutIds: WebSendMessageTimeoutId[],
): void {
  const { assistantMessage, effectiveExecutionPolicy, input, memoryEnabled, session, userMessage } = context;
  const agent = mockAgents.find((candidate) => candidate.id === session.agentId);
  if (agent?.launch.kind !== "api" || effectiveExecutionPolicy === "readonly") return;

  const isTrusted = isAgentAutoApproved(session.agentId);
  const callId = `web-tool-approval-${assistantMessage.id}`;
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    if (isTrusted) {
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: {
          id: callId,
          name: "shell",
          input: { command: "echo mock" },
          output: "mock\n",
          status: "completed",
        },
      });
      return;
    }
    createWebPendingApproval(callId, {
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolName: "shell",
      agentId: session.agentId,
      action: "shell.exec",
      resource: "workspace",
      riskLevel: "L2",
      createdAt: new Date().toISOString(),
    });
    publishWebChatEvent({
      type: "tool_use",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolUse: {
        id: callId,
        name: "shell",
        input: { command: "echo mock" },
        status: "awaiting_approval",
      },
    });
  }, 230);

  if (input.content.includes(WEB_MOCK_QUESTION_TRIGGER)) {
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: {
          id: `web-tool-question-${assistantMessage.id}`,
          name: "ask_user_question",
          input: {
            question: "Which approach should the simulated agent take?",
            options: ["Rewrite the module", "Patch it in place"],
          },
          status: "awaiting_input",
        },
      });
    }, 240);
  }
  if (input.content.includes(WEB_MOCK_PLAN_EXIT_TRIGGER)) {
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: {
          id: `web-tool-plan-exit-${assistantMessage.id}`,
          name: "exit_plan_mode",
          input: { plan: "Rewrite the parser, then update its three callers." },
          status: "awaiting_input",
        },
      });
    }, 245);
  }
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    publishWebChatEvent({
      type: "tool_use",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolUse: {
        id: `web-grep-${assistantMessage.id}`,
        name: "grep",
        input: { pattern: "export function", output_mode: "files_with_matches" },
        output: "src/App.tsx\nsrc/main.tsx",
        status: "completed",
      },
    });
  }, 233);
  if (memoryEnabled) {
    scheduleWebSendMessageTimeout(timeoutIds, () => {
      const memory = createWebAgentMemory(
        session.agentId,
        session.folder,
        `User said: "${userMessage.content.slice(0, 60)}"`,
        "explicit",
      );
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: {
          id: `web-remember-${assistantMessage.id}`,
          name: "remember",
          input: { content: memory.content },
          output: "Saved.",
          status: "completed",
        },
      });
    }, 235);
  }
  scheduleMcpApproval(context, timeoutIds);
}

function scheduleMcpApproval(context: WebSendMessageContext, timeoutIds: WebSendMessageTimeoutId[]): void {
  const { assistantMessage, input, session } = context;
  const callId = `web-tool-approval-mcp-${assistantMessage.id}`;
  const simulation = createWebMcpToolSimulationPlan({
    callId,
    catalog: [{
      name: "mcp__mock-server__search",
      description: "Search deterministic Web preview data",
      inputSchema: { type: "object", properties: { query: { type: "string" } } },
    }],
    toolName: "mcp__mock-server__search",
    arguments: { query: "mock" },
    result: "mock MCP result",
  });
  scheduleWebSendMessageTimeout(timeoutIds, () => {
    if (!simulation.success) {
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: simulation.failed,
      });
      return;
    }
    createWebPendingApproval(callId, {
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolName: simulation.awaitingApproval.name,
      input: simulation.completed.input,
      output: simulation.completed.output,
      agentId: session.agentId,
      action: "mcp.tool",
      resource: simulation.awaitingApproval.name,
      riskLevel: "L2",
      createdAt: new Date().toISOString(),
    });
    publishWebChatEvent({
      type: "tool_use",
      sessionId: input.sessionId,
      messageId: assistantMessage.id,
      toolUse: simulation.awaitingApproval,
    });
  }, 237);
}
