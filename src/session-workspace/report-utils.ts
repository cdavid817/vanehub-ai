import type { ChatMessage, MessageStatus } from "../types/chat";

export interface SessionReportTimelineItem {
  id: string;
  kind: "message" | "tool" | "failure" | "completion";
  label: string;
  timestamp: string;
}

export interface SessionReport {
  reportedInputTokens: number;
  reportedOutputTokens: number;
  estimatedInputCharacters: number;
  estimatedOutputCharacters: number;
  messageCount: number;
  failedCount: number;
  statusCounts: Record<MessageStatus, number>;
  toolRanking: Array<{ name: string; count: number }>;
  timeline: SessionReportTimelineItem[];
}

export function aggregateSessionReport(messages: ChatMessage[]): SessionReport {
  let reportedInputTokens = 0;
  let reportedOutputTokens = 0;
  let estimatedInputCharacters = 0;
  let estimatedOutputCharacters = 0;
  let failedCount = 0;
  const statusCounts: Record<MessageStatus, number> = {
    pending: 0,
    streaming: 0,
    completed: 0,
    failed: 0,
    cancelled: 0,
  };
  const toolCounts = new Map<string, number>();
  const timeline: SessionReportTimelineItem[] = [];

  for (const message of messages) {
    if (message.tokenUsage) {
      reportedInputTokens += message.tokenUsage.input;
      reportedOutputTokens += message.tokenUsage.output;
    } else if (message.role === "user") {
      estimatedInputCharacters += message.content.length;
    } else if (message.role === "assistant" && message.status === "completed") {
      estimatedOutputCharacters += message.content.length;
    }
    statusCounts[message.status] += 1;
    if (message.status === "failed") failedCount += 1;
    timeline.push({
      id: message.id,
      kind: message.status === "failed" ? "failure" : "message",
      label: message.role,
      timestamp: message.createdAt,
    });
    if (message.status === "completed") {
      timeline.push({
        id: `${message.id}-completion`,
        kind: "completion",
        label: message.role,
        timestamp: message.updatedAt,
      });
    }
    for (const tool of message.toolUse ?? []) {
      toolCounts.set(tool.name, (toolCounts.get(tool.name) ?? 0) + 1);
      timeline.push({
        id: `${message.id}-${tool.id}`,
        kind: tool.status === "failed" ? "failure" : "tool",
        label: tool.name,
        timestamp: message.updatedAt,
      });
    }
  }

  return {
    reportedInputTokens,
    reportedOutputTokens,
    estimatedInputCharacters,
    estimatedOutputCharacters,
    messageCount: messages.length,
    failedCount,
    statusCounts,
    toolRanking: [...toolCounts.entries()]
      .map(([name, count]) => ({ name, count }))
      .sort((left, right) => right.count - left.count || left.name.localeCompare(right.name)),
    timeline: timeline.sort((left, right) => left.timestamp.localeCompare(right.timestamp)),
  };
}
