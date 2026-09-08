// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../../i18n";
import { agentService } from "../../services/runtime-agent-client";
import { settingsPages } from "../settings-pages";
import { EvaluationPage } from "./evaluation-page";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("EvaluationPage", () => {
  it("hosts the unchanged evaluation center under the Agent settings group", async () => {
    await i18n.changeLanguage("en");
    vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationTasks").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
    render(<EvaluationPage />);

    expect(screen.getByTestId("evaluation-settings-page").querySelector("[data-testid='evaluation-center']")).toBeTruthy();
    expect(screen.getByTestId("evaluation-run")).toBeTruthy();

    const page = settingsPages.find((candidate) => candidate.id === "evaluation");
    expect(page).toMatchObject({ group: "agent", labelKey: "settings.pages.evaluation", searchPlaceholderKey: "settings.search.evaluation" });
    expect(settingsPages.findIndex((candidate) => candidate.id === "evaluation")).toBe(settingsPages.findIndex((candidate) => candidate.id === "cli-parameters") + 1);
  });
});
