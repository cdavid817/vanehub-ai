export const promptHookVirtualizationThreshold = 500;

export function shouldVirtualizePromptHooks(count: number) {
  return count > promptHookVirtualizationThreshold;
}

export function chunkItems<T>(items: readonly T[], size: number): T[][] {
  if (!Number.isInteger(size) || size < 1) {
    throw new Error("Virtual row size must be a positive integer.");
  }

  const rows: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    rows.push(items.slice(index, index + size));
  }
  return rows;
}
