export function retainAsyncCleanup(
  disposed: boolean,
  cleanup: () => void,
): (() => void) | null {
  if (disposed) {
    cleanup();
    return null;
  }
  return cleanup;
}
