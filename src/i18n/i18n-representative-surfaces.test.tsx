// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useTranslation } from "react-i18next";
import { afterEach, describe, expect, it } from "vitest";
import { MessageItem } from "../components/chat/MessageItem";
import { NotificationCenter } from "../notifications/notification-center";
import { NotificationProvider } from "../notifications/notification-provider";
import { CliEnvironmentCard } from "../settings/pages/cli-environment-card";
import type { CliEnvironmentSnapshot } from "../types/cli-environment-snapshot";
import type { ChatMessage } from "../types/chat";
import { activateAppLanguage } from ".";
import { loadLocaleResource, supportedLocales } from "./supported-locales";

const staticRepresentativeKeys = [
  "settings.pages.basic",
  "basic.title",
  "app.error.title",
  "sessionTabs.state.empty",
] as const;

const message: ChatMessage = {
  id: "message-localized-surface",
  sessionId: "session-localized-surface",
  role: "assistant",
  content: "Localized chat surface",
  status: "completed",
  createdAt: "2026-07-17T08:30:00.000Z",
  updatedAt: "2026-07-17T08:30:00.000Z",
  sessionSequence: 1,
  executionRunId: null,
};

const cliSnapshot: CliEnvironmentSnapshot = {
  schemaVersion: 1,
  agentId: "codex-cli",
  displayName: "OpenAI Codex CLI",
  provider: "OpenAI",
  executableNames: ["codex"],
  scope: "local-desktop",
  overallState: "conflict",
  freshness: "fresh",
  environmentFingerprint: "fingerprint-a",
  installations: [{
    id: "codex",
    executablePath: "/mock/bin/codex",
    canonicalPath: null,
    aliasPaths: [],
    targetMissing: false,
    reportedVersion: "1.2.0",
    sourceId: "npm",
    sourceKind: "npm",
    sourceConfidence: "inferred",
    pathPriority: 0,
    environmentOrigin: "path",
    executableStatus: "healthy",
  }],
  pathSelectedInstallationId: "codex",
  recommendedInstallationId: "codex",
  discovery: "found-one",
  executable: "healthy",
  authentication: "unknown",
  readiness: "unknown",
  compatibility: "unknown",
  update: "available",
  conflicts: [{
    kind: "path-shadowing",
    severity: "blocking",
    installationIds: ["codex"],
    blocksMutation: true,
    blocksLaunch: false,
    reasonCode: "path-shadowing",
  }],
  sources: [],
  allowedActions: [],
  lastMutation: null,
  lastOperationId: null,
  checkedAt: "2026-07-17T08:30:00.000Z",
};

function RepresentativeSurfaces() {
  const { t } = useTranslation();
  return (
    <main>
      {staticRepresentativeKeys.map((key) => <span data-testid={key} key={key}>{t(key)}</span>)}
      <MessageItem message={message} />
      <NotificationProvider>
        <NotificationCenter />
      </NotificationProvider>
      <CliEnvironmentCard
        diagnosticsExpanded={false}
        mutating={false}
        operationExpanded={false}
        refreshing={false}
        selectedVersion=""
        snapshot={cliSnapshot}
        onRefresh={() => undefined}
        onRequestChange={() => undefined}
        onSelectedVersionChange={() => undefined}
        onToggleDiagnostics={() => undefined}
        onToggleOperation={() => undefined}
      />
    </main>
  );
}

describe("representative localized surfaces", () => {
  afterEach(cleanup);

  for (const locale of supportedLocales) {
    it(`renders ${locale.id} navigation, settings, chat, notification, dialog, error, and empty-state copy from its own resource`, async () => {
      const resource = await loadLocaleResource(locale.id);
      await activateAppLanguage(locale.id);
      render(<RepresentativeSurfaces />);

      for (const key of staticRepresentativeKeys) {
        expect(resource[key]).toBeTruthy();
        expect(screen.getByTestId(key).textContent).toBe(resource[key]);
      }
      expect(screen.getByText(resource["chat.status.completed"])).toBeTruthy();
      expect(screen.getByText(resource["chat.agent"])).toBeTruthy();

      fireEvent.click(screen.getByRole("button", { name: resource["layout.notifications"] }));
      expect(screen.getByRole("dialog", { name: resource["layout.notifications"] })).toBeTruthy();
      expect(screen.getByText(resource["notifications.empty"])).toBeTruthy();
      expect(screen.getByText(resource["notifications.emptyDescription"])).toBeTruthy();

      // A blocking conflict is explained from its localized code, not from a parsed message.
      expect(screen.getByText(resource["cli.conflict.path-shadowing"])).toBeTruthy();
      expect(screen.getByRole("button", {
        name: resource["cli.refreshOne"].replace("{{name}}", cliSnapshot.displayName),
      })).toBeTruthy();
      expect(document.documentElement.lang).toBe(locale.id);
      expect(document.documentElement.dir).toBe(locale.direction);
    });
  }
});
