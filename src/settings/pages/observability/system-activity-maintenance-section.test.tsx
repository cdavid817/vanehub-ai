// @vitest-environment jsdom

import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import { resetWebSystemActivityForTest, seedWebSystemActivityEventForTest } from "../../../services/web-system-activity-state";
import { renderWithAppProviders } from "../../../test/render";
import { SystemActivityMaintenanceSection } from "./system-activity-maintenance-section";

beforeEach(async () => {
  resetWebSystemActivityForTest();
  await activateAppLanguage("en");
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("SystemActivityMaintenanceSection", () => {
  it("shows an empty state before any system session exists", async () => {
    renderWithAppProviders(<SystemActivityMaintenanceSection />);
    expect(await screen.findByTestId("observability-system-activity-empty")).toBeTruthy();
    expect(screen.getByText("System activity")).toBeTruthy();
  });

  it("hosts the health panel and the export, rebuild, and preference controls for the chosen session", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    seedWebSystemActivityEventForTest("global", "global", "skill_created");
    const exportSpy = vi.spyOn(agentService, "exportSystemActivity");
    renderWithAppProviders(<SystemActivityMaintenanceSection />);

    const picker = await screen.findByRole("combobox", { name: "System session" });
    expect(await screen.findByTestId("system-activity-health")).toBeTruthy();
    expect(await screen.findByTestId("system-activity-rebuild")).toBeTruthy();
    expect(screen.getByTestId("system-activity-export")).toBeTruthy();
    expect(await screen.findByTestId("system-activity-preferences")).toBeTruthy();

    await userEvent.selectOptions(picker, (picker as HTMLSelectElement).options[1].value);
    await waitFor(() => expect((picker as HTMLSelectElement).selectedIndex).toBe(1));
    // The same service methods the System Activity surface called: choosing a target and exporting
    // goes through the boundary, not through anything the settings page owns.
    await userEvent.click(screen.getByRole("button", { name: "Choose location" }));
    await waitFor(() => expect((screen.getByLabelText("Export file path") as HTMLInputElement).value).not.toBe(""));
    await userEvent.click(screen.getByTestId("system-activity-export"));
    await waitFor(() => expect(exportSpy).toHaveBeenCalledOnce());
  });
});
