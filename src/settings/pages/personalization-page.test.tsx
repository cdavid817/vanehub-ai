// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { SettingsProvider } from "../settings-provider";
import { PersonalizationPage } from "./personalization-page";

/** `SettingsProvider` renders nothing until settings load, so every test waits for the shell. */
async function renderPage() {
  const rendered = renderWithAppProviders(
    <SettingsProvider>
      <PersonalizationPage />
    </SettingsProvider>,
  );
  await screen.findByRole("tablist");
  return rendered;
}

describe("PersonalizationPage", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("offers all four destinations", async () => {
    await renderPage();

    for (const view of ["overview", "instructions", "memory", "runtimePreview"]) {
      expect(screen.getByTestId(`personalization-view-tab-${view}`)).toBeTruthy();
    }
  });

  it("opens on Overview", async () => {
    await renderPage();

    expect(screen.getByTestId("personalization-view-tab-overview").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("personalization-overview-empty")).toBeTruthy();
  });

  it("mounts only the selected view", async () => {
    await renderPage();

    // The Memory view issues its own queries; leaving every view mounted would fetch on each
    // visit to the page regardless of which destination the user opened.
    expect(screen.queryByTestId("personalization-runtime-preview-empty")).toBeNull();

    await userEvent.click(screen.getByTestId("personalization-view-tab-runtimePreview"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-runtime-preview-empty")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-overview-empty")).toBeNull();
  });

  it("marks the selected destination for assistive technology", async () => {
    await renderPage();

    await userEvent.click(screen.getByTestId("personalization-view-tab-memory"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-view-tab-memory").getAttribute("aria-selected")).toBe("true");
    });
    expect(screen.getByTestId("personalization-view-tab-overview").getAttribute("aria-selected")).toBe("false");
  });

  it("says plainly that a CLI's own compaction is not VaneHub's to manage", async () => {
    await renderPage();

    await userEvent.click(screen.getByTestId("personalization-view-tab-runtimePreview"));

    // A preview that stayed silent about this would leave a user believing the estimate covers
    // their whole session.
    await waitFor(() => {
      expect(screen.getByText(/VaneHub/u)).toBeTruthy();
    });
  });
});
