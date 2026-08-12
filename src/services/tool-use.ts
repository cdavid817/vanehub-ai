import type { ToolUseBlock } from "../types/chat";

export function upsertToolUse(
  current: readonly ToolUseBlock[],
  update: ToolUseBlock,
): ToolUseBlock[] {
  const index = current.findIndex((tool) => tool.id === update.id);
  if (index === -1) return [...current, update];

  return current.map((tool, toolIndex) => {
    if (toolIndex !== index) return tool;
    return {
      ...tool,
      ...update,
      input: update.input === undefined ? tool.input : update.input,
      output: update.output === undefined ? tool.output : update.output,
    };
  });
}

export function normalizeToolUse(toolUse: readonly ToolUseBlock[]): ToolUseBlock[] {
  return toolUse.reduce<ToolUseBlock[]>(upsertToolUse, []);
}
