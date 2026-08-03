import { describe, expect, it } from "vitest";
import { MCP_LIMITS, type McpToolInfo } from "../types/mcp";
import { createWebMcpToolSimulationPlan, type WebMcpToolSimulationInput } from "./web-mcp-tool-simulation";

const baseInput: WebMcpToolSimulationInput = {
  callId: "web-mcp-call",
  catalog: [{ name: "mcp__mock-server__search", description: "Search mock data", inputSchema: { type: "object" } }],
  toolName: "mcp__mock-server__search",
  arguments: { query: "mock" },
  result: "mock result",
};

describe("Web MCP tool simulation", () => {
  it("creates the approval and completion events for a bounded MCP call", () => {
    const plan = createWebMcpToolSimulationPlan(baseInput);

    expect(plan).toMatchObject({
      success: true,
      awaitingApproval: { name: baseInput.toolName, status: "awaiting_approval" },
      completed: { name: baseInput.toolName, output: baseInput.result, status: "completed" },
    });
  });

  it.each([
    [
      "catalog",
      {
        catalog: Array.from(
          { length: MCP_LIMITS.toolsPerServer + 1 },
          (_, index): McpToolInfo => ({ name: `mcp__mock__tool_${index}` }),
        ),
      },
      "catalog",
    ],
    ["arguments", { arguments: nestedObject(MCP_LIMITS.jsonDepth + 1) }, "arguments"],
    ["result", { result: "x".repeat(MCP_LIMITS.toolResultBytes + 1) }, "result"],
  ] as const)("returns a safe failure without native-effect events for oversized %s", (_case, override, field) => {
    const plan = createWebMcpToolSimulationPlan({ ...baseInput, ...override });

    expect(plan).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field,
      failed: {
        status: "failed",
        output: { errorCode: "limit_exceeded" },
      },
    });
    expect(plan).not.toHaveProperty("awaitingApproval");
    expect(plan).not.toHaveProperty("completed");
  });
});

function nestedObject(depth: number): Record<string, unknown> {
  let value: unknown = null;
  for (let index = 1; index < depth; index += 1) value = { nested: value };
  return value as Record<string, unknown>;
}
