import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement, ReactNode } from "react";
import { I18nextProvider } from "react-i18next";
import { i18n } from "../i18n";
import type { AgentService } from "../services/agent-service";

export function renderWithAppProviders(
  ui: ReactElement,
  options: { queryClient?: QueryClient; theme?: "futuristic" | "minimal" } = {},
) {
  const queryClient = options.queryClient ?? new QueryClient({
    defaultOptions: {
      mutations: { retry: false },
      queries: { retry: false },
    },
  });
  document.documentElement.dataset.theme = options.theme ?? "futuristic";
  const rendered = render(
    <TestProviders queryClient={queryClient}>{ui}</TestProviders>,
  );
  return { ...rendered, queryClient, user: userEvent.setup() };
}

export function createAgentServiceDouble(overrides: Partial<AgentService>): AgentService {
  return new Proxy({} as AgentService, {
    get(_target, property) {
      const implementation = Reflect.get(overrides, property);
      if (implementation !== undefined) return implementation;
      return () => Promise.reject(new Error(`Unexpected AgentService call: ${String(property)}`));
    },
  });
}

function TestProviders({ children, queryClient }: { children: ReactNode; queryClient: QueryClient }) {
  return (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </I18nextProvider>
  );
}
