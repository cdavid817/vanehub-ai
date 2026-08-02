import type { ClientLogEvent } from "./types/settings";

export interface BootstrapFailureCopy {
  title: string;
  description: string;
  retry: string;
}

interface BootstrapFailureOptions {
  root: HTMLElement;
  copy: BootstrapFailureCopy;
  error: unknown;
  surface: "main" | "floating-assistant";
  retry: () => void;
  report: (event: ClientLogEvent) => Promise<void>;
}

function normalizeError(error: unknown): { message: string; stack?: string } {
  if (error instanceof Error) {
    return { message: error.message, stack: error.stack };
  }
  return { message: String(error) };
}

export function createBootstrapFailureEvent(
  error: unknown,
  surface: BootstrapFailureOptions["surface"],
): ClientLogEvent {
  const normalized = normalizeError(error);
  return {
    level: "error",
    kind: "critical-operation-failure",
    message: normalized.message,
    source: "frontend-bootstrap",
    details: { surface },
    stack: normalized.stack,
  };
}

export function renderBootstrapFailure(
  root: HTMLElement,
  copy: BootstrapFailureCopy,
  retry: () => void,
): void {
  const main = document.createElement("main");
  main.className = "flex min-h-screen items-center justify-center bg-background p-6 text-foreground";
  main.dataset.bootstrapRecovery = "true";

  const panel = document.createElement("section");
  panel.className = "ucd-panel w-full max-w-lg rounded-lg p-5";

  const title = document.createElement("h1");
  title.className = "text-base font-semibold";
  title.textContent = copy.title;

  const description = document.createElement("p");
  description.className = "mt-2 text-sm text-muted-foreground";
  description.textContent = copy.description;

  const retryButton = document.createElement("button");
  retryButton.className = "mt-4 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground";
  retryButton.type = "button";
  retryButton.textContent = copy.retry;
  retryButton.addEventListener("click", retry);

  panel.append(title, description, retryButton);
  main.append(panel);
  root.replaceChildren(main);
}

export function recoverFromBootstrapFailure(options: BootstrapFailureOptions): void {
  renderBootstrapFailure(options.root, options.copy, options.retry);

  // Recovery must remain available even when the runtime adapter cannot load or persist diagnostics.
  void Promise.resolve()
    .then(() => options.report(createBootstrapFailureEvent(options.error, options.surface)))
    .catch(() => undefined);
}
