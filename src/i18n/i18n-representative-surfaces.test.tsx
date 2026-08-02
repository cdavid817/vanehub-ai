// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useTranslation } from "react-i18next";
import { afterEach, describe, expect, it } from "vitest";
import { MessageItem } from "../components/chat/MessageItem";
import { NotificationCenter } from "../notifications/notification-center";
import { NotificationProvider } from "../notifications/notification-provider";
import { CliConflictDialog } from "../settings/pages/cli-conflict-dialog";
import type { CliToolStatus } from "../types/agent";
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
};

const cliTool: CliToolStatus = {
  agentId: "codex-cli",
  displayName: "OpenAI Codex CLI",
  provider: "OpenAI",
  executableName: "codex",
  packageName: "@openai/codex",
  installed: true,
  currentVersion: "1.2.0",
  latestVersion: "1.3.0",
  availableVersions: ["1.3.0", "1.2.0"],
  detectedPath: "C:\\Users\\dev\\codex.cmd",
  installCommand: "npm install -g @openai/codex@latest",
  lastCheckedAt: "2026-07-17T08:30:00.000Z",
  lastError: null,
  lastOperationId: null,
  versionCheckStatus: "succeeded",
  environmentType: "windows",
  installations: [{
    path: "C:\\Users\\dev\\codex.cmd",
    version: "1.2.0",
    runnable: true,
    error: null,
    source: "npm",
    environmentType: "windows",
    isActive: true,
  }],
  activeInstallationPath: "C:\\Users\\dev\\codex.cmd",
  conflictState: "multiple",
  lifecycleEligibility: "npm",
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
      <CliConflictDialog onCancel={() => undefined} onConfirm={() => undefined} tool={cliTool} />
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

      const conflictTitle = resource["cli.confirm.title"].replace("{{name}}", cliTool.displayName);
      expect(screen.getByRole("dialog", { name: conflictTitle })).toBeTruthy();
      expect(screen.getByRole("button", { name: resource["cli.confirm.continue"] })).toBeTruthy();
      expect(document.documentElement.lang).toBe(locale.id);
      expect(document.documentElement.dir).toBe(locale.direction);
    });
  }
});
