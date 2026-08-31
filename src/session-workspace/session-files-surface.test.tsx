// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { SessionFilesSurface } from "./session-files-surface";

vi.mock("./files-tab", () => ({
  FilesTab: () => <div data-testid="explorer-view" />,
}));
vi.mock("./documents-tab", () => ({
  DocumentsTab: () => <div data-testid="documents-view" />,
}));

function mount() {
  return render(
    <I18nextProvider i18n={i18n}>
      <SessionFilesSurface sessionId="session-1" />
    </I18nextProvider>,
  );
}

describe("SessionFilesSurface", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("opens on the Explorer view without mounting Documents yet", async () => {
    mount();
    expect(await screen.findByTestId("explorer-view")).toBeTruthy();
    expect(screen.queryByTestId("documents-view")).toBeNull();
  });

  it("switches to Documents and keeps Explorer mounted behind it", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("explorer-view");

    await user.click(screen.getByRole("tab", { name: i18n.t("filesSurface.view.documents") }));

    expect(await screen.findByTestId("documents-view")).toBeTruthy();
    // Merging Documents into Files must not cost the Explorer its mounted state — design.md
    // Decision 7 requires "existing document and file service behavior SHALL remain reachable".
    expect(screen.getByTestId("explorer-view")).toBeTruthy();
  });

  it("marks the active view as selected and the other as not", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("explorer-view");

    const explorerTab = screen.getByRole("tab", { name: i18n.t("filesSurface.view.explorer") });
    const documentsTab = screen.getByRole("tab", { name: i18n.t("filesSurface.view.documents") });
    expect(explorerTab.getAttribute("aria-selected")).toBe("true");
    expect(documentsTab.getAttribute("aria-selected")).toBe("false");

    await user.click(documentsTab);

    expect(explorerTab.getAttribute("aria-selected")).toBe("false");
    expect(documentsTab.getAttribute("aria-selected")).toBe("true");
  });
});
