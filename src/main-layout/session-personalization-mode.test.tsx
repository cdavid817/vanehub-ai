// @vitest-environment jsdom

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { I18nextProvider } from "react-i18next";
import { i18n } from "../i18n";
import {
  modeForWorkspace,
  SessionPersonalizationModeSelector,
} from "./session-personalization-mode-selector";
import { SessionPersonalizationBadge } from "../session-workspace/session-personalization-badge";

function renderSelector(hasWorkspace: boolean, mode: "standard" | "project-only" | "temporary" = "standard") {
  const onChange = vi.fn();
  render(
    <I18nextProvider i18n={i18n}>
      <SessionPersonalizationModeSelector hasWorkspace={hasWorkspace} mode={mode} onChange={onChange} />
    </I18nextProvider>,
  );
  return onChange;
}

describe("session personalization mode", () => {
  it("offers all three modes", async () => {
    renderSelector(true);

    const select = screen.getByTestId("session-personalization-mode");
    for (const label of ["标准", "仅本项目", "临时"]) {
      expect(within(select).getByText(label)).toBeTruthy();
    }
  });

  it("disables project-only without a workspace and says why", () => {
    renderSelector(false);

    const option = within(screen.getByTestId("session-personalization-mode")).getByText("仅本项目");
    // Hidden instead of disabled would leave a user who was told the mode exists unable to find it.
    expect(option.hasAttribute("disabled")).toBe(true);
    expect(screen.getByTestId("session-personalization-mode-blocked")).toBeTruthy();
  });

  it("points the select at the explanation so a screen reader hears it", () => {
    renderSelector(false);

    const select = screen.getByTestId("session-personalization-mode");
    const explanation = screen.getByTestId("session-personalization-mode-blocked");
    expect(select.getAttribute("aria-describedby")).toBe(explanation.getAttribute("id"));
  });

  it("drops the explanation once a workspace exists", () => {
    renderSelector(true);

    expect(screen.queryByTestId("session-personalization-mode-blocked")).toBeNull();
    expect(
      screen.getByTestId("session-personalization-mode").getAttribute("aria-describedby"),
    ).toBeNull();
  });

  it("explains what each mode does before the session is created", async () => {
    renderSelector(true, "temporary");

    expect(screen.getByTestId("session-personalization-mode-help").textContent).toContain(
      "不会被记住",
    );
  });

  it("reports the chosen mode", async () => {
    const onChange = renderSelector(true);

    await userEvent.selectOptions(screen.getByTestId("session-personalization-mode"), "temporary");

    await waitFor(() => {
      expect(onChange).toHaveBeenCalledWith("temporary");
    });
  });

  it("corrects project-only back to standard when the workspace goes away", () => {
    // The store refuses the combination, so correcting here is what stops a submit failing against
    // a control the user can no longer see.
    expect(modeForWorkspace("project-only", false)).toBe("standard");
    expect(modeForWorkspace("project-only", true)).toBe("project-only");
    expect(modeForWorkspace("temporary", false)).toBe("temporary");
  });

  it("badges a restricted session and says what it retains", () => {
    render(
      <I18nextProvider i18n={i18n}>
        <SessionPersonalizationBadge mode="temporary" />
      </I18nextProvider>,
    );

    const badge = screen.getByTestId("session-personalization-badge-temporary");
    expect(badge.textContent).toBe("临时");
    expect(badge.getAttribute("title")).toContain("不使用也不记录");
  });

  it("badges nothing for a standard session", () => {
    render(
      <I18nextProvider i18n={i18n}>
        <SessionPersonalizationBadge mode="standard" />
      </I18nextProvider>,
    );

    // A badge on every session becomes furniture, and the one that matters is the one saying this
    // conversation is not being remembered the way the others are.
    expect(screen.queryByTestId("session-personalization-badge-standard")).toBeNull();
  });
});
