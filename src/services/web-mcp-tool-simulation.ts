import type { ToolUseBlock } from "../types/chat";
import type { McpToolInfo } from "../types/mcp";
import { validateMcpToolArguments, type McpValidationFailure } from "./mcp-validation";
import { validateMcpToolCatalog, validateMcpToolResult } from "./mcp-tool-validation";

export interface WebMcpToolSimulationInput {
  callId: string;
  catalog: McpToolInfo[];
  toolName: string;
  arguments: Record<string, unknown>;
  result: string;
}

export type WebMcpToolSimulationPlan =
  | {
      success: true;
      awaitingApproval: ToolUseBlock;
      completed: ToolUseBlock;
    }
  | (McpValidationFailure & {
      failed: ToolUseBlock;
    });

export function createWebMcpToolSimulationPlan(input: WebMcpToolSimulationInput): WebMcpToolSimulationPlan {
  const validation = firstFailure(
    validateMcpToolCatalog(input.catalog),
    validateMcpToolArguments(input.arguments),
    validateMcpToolResult(input.result),
  );
  if (validation) {
    return {
      ...validation,
      failed: {
        id: input.callId,
        name: input.toolName,
        input: input.arguments,
        output: { errorCode: validation.errorCode, message: validation.message },
        status: "failed",
      },
    };
  }

  if (!input.catalog.some((tool) => tool.name === input.toolName)) {
    const missing: McpValidationFailure = {
      success: false,
      errorCode: "validation",
      field: "catalog",
      message: "Simulated MCP tool is not present in the bounded catalog",
    };
    return {
      ...missing,
      failed: {
        id: input.callId,
        name: input.toolName,
        input: input.arguments,
        output: { errorCode: missing.errorCode, message: missing.message },
        status: "failed",
      },
    };
  }

  return {
    success: true,
    awaitingApproval: {
      id: input.callId,
      name: input.toolName,
      input: input.arguments,
      status: "awaiting_approval",
    },
    completed: {
      id: input.callId,
      name: input.toolName,
      input: input.arguments,
      output: input.result,
      status: "completed",
    },
  };
}

function firstFailure(...results: ReturnType<typeof validateMcpToolCatalog>[]): McpValidationFailure | null {
  return results.find((result): result is McpValidationFailure => !result.success) ?? null;
}
