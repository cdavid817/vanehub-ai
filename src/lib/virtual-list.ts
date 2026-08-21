export const promptHookVirtualizationThreshold = 500;

export function shouldVirtualizePromptHooks(count: number) {
  return count > promptHookVirtualizationThreshold;
}
