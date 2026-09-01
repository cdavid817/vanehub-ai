// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { CreateSessionStep4 } from "./create-session-step-4";
import { createInitialCreateSessionDraft } from "./create-session-draft-model";
import type { CreateSessionValidation } from "./create-session-validation";

const noErrors: CreateSessionValidation = {
  canSubmit: true,
  agent: null,
  workspace: null,
  seats: null,
  sshConnection: null,
};

describe("CreateSessionStep4's Review-level error summary (task 11.10)", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders no summary when the draft is fully valid", () => {
    render(
      <CreateSessionStep4
        draft={createInitialCreateSessionDraft()}
        effectivePersonalizationMode="standard"
        onGoToStep={vi.fn()}
        onTitleChange={vi.fn()}
        selectedAgent={null}
        validation={noErrors}
      />,
    );
    expect(screen.queryByText("Resolve these before creating the session:")).toBeNull();
  });

  it("lists an active agent error and jumps to Step 2 when Fix is pressed", () => {
    const onGoToStep = vi.fn();
    render(
      <CreateSessionStep4
        draft={createInitialCreateSessionDraft()}
        effectivePersonalizationMode="standard"
        onGoToStep={onGoToStep}
        onTitleChange={vi.fn()}
        selectedAgent={null}
        validation={{ ...noErrors, agent: "agent-unselectable", canSubmit: false }}
      />,
    );
    expect(screen.getByText("Resolve these before creating the session:")).toBeTruthy();
    expect(screen.getByText("Choose an Agent that is installed and available.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    expect(onGoToStep).toHaveBeenCalledWith(2);
  });

  it("lists an active workspace error and jumps to Step 3 when Fix is pressed", () => {
    const onGoToStep = vi.fn();
    render(
      <CreateSessionStep4
        draft={createInitialCreateSessionDraft()}
        effectivePersonalizationMode="standard"
        onGoToStep={onGoToStep}
        onTitleChange={vi.fn()}
        selectedAgent={null}
        validation={{ ...noErrors, workspace: "workspace-path-missing", canSubmit: false }}
      />,
    );
    expect(screen.getByText("Enter a project path.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fix" }));
    expect(onGoToStep).toHaveBeenCalledWith(3);
  });

  it("lists both agent and seats errors independently when somehow both are active", () => {
    render(
      <CreateSessionStep4
        draft={createInitialCreateSessionDraft()}
        effectivePersonalizationMode="standard"
        onGoToStep={vi.fn()}
        onTitleChange={vi.fn()}
        selectedAgent={null}
        validation={{ ...noErrors, agent: "agent-unselectable", seats: "seats-too-few", canSubmit: false }}
      />,
    );
    expect(screen.getAllByRole("button", { name: "Fix" })).toHaveLength(2);
  });
});
