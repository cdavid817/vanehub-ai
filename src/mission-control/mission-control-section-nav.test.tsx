// @vitest-environment jsdom

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { MissionControlFacet, MissionControlFacetState } from "../types/mission-control";
import {
  MissionControlSectionNav,
  MissionControlSectionNavView,
  type MissionControlSectionNavProps,
} from "./mission-control-section-nav";

afterEach(() => cleanup());

const STATES: Partial<Record<MissionControlFacet, MissionControlFacetState>> = {
  files: "unavailable",
  logs: "restricted",
};

function availability(facet: MissionControlFacet): MissionControlFacetState {
  return STATES[facet] ?? "available";
}

function ControlledNav({ compact }: { compact?: boolean }) {
  const [activeFacet, setActiveFacet] = useState<MissionControlFacet>("overview");
  const props: MissionControlSectionNavProps = { activeFacet, availability, onSelect: setActiveFacet };
  return compact === undefined
    ? <MissionControlSectionNav {...props} />
    : <MissionControlSectionNavView {...props} compact={compact} />;
}

describe("MissionControlSectionNavView", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders a readable tablist with all nine facets when not compact", () => {
    render(<ControlledNav compact={false} />);
    const tablist = screen.getByRole("tablist", { name: "Run detail sections" });
    expect(tablist).toBeTruthy();
    for (const label of ["Overview", "Timeline", "Tools", "Files / Artifacts", "Review", "Tests / Verification", "Context", "Usage", "Logs"]) {
      expect(screen.getByRole("tab", { name: new RegExp(`^${label.replace("/", "\\/")}`) })).toBeTruthy();
    }
  });

  it("marks the active facet as selected, and disables/labels unavailable and restricted facets", () => {
    render(<ControlledNav compact={false} />);
    expect(screen.getByRole("tab", { name: "Overview" }).getAttribute("aria-selected")).toBe("true");
    const files = screen.getByRole("tab", { name: /^Files \/ Artifacts/ });
    expect(files.getAttribute("aria-disabled")).toBe("true");
    expect(files.textContent).toContain("Unavailable");
    const logs = screen.getByRole("tab", { name: /^Logs/ });
    expect(logs.textContent).toContain("Restricted");
  });

  it("selects a facet on click", () => {
    render(<ControlledNav compact={false} />);
    fireEvent.click(screen.getByRole("tab", { name: "Timeline" }));
    expect(screen.getByRole("tab", { name: "Timeline" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tab", { name: "Overview" }).getAttribute("aria-selected")).toBe("false");
  });

  it("supports roving-tabindex keyboard navigation (ArrowRight/ArrowLeft/Home/End)", () => {
    render(<ControlledNav compact={false} />);
    const overview = screen.getByRole("tab", { name: "Overview" });
    expect(overview.getAttribute("tabIndex")).toBe("0");

    fireEvent.keyDown(overview, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Timeline" }).getAttribute("aria-selected")).toBe("true");

    fireEvent.keyDown(screen.getByRole("tab", { name: "Timeline" }), { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { name: "Overview" }).getAttribute("aria-selected")).toBe("true");

    // "Logs" is one of this file's own restricted fixtures (STATES above), so its accessible name
    // folds in the " · Restricted" suffix -- matched with a prefix regex for the same reason
    // "Files / Artifacts" is elsewhere in this file.
    fireEvent.keyDown(screen.getByRole("tab", { name: "Overview" }), { key: "End" });
    expect(screen.getByRole("tab", { name: /^Logs/ }).getAttribute("aria-selected")).toBe("true");

    fireEvent.keyDown(screen.getByRole("tab", { name: /^Logs/ }), { key: "Home" });
    expect(screen.getByRole("tab", { name: "Overview" }).getAttribute("aria-selected")).toBe("true");
  });

  it("renders a labeled select with all nine facets when compact, disabling unavailable/restricted options", () => {
    render(<ControlledNav compact={true} />);
    const select = screen.getByRole("combobox", { name: "Run detail sections" }) as HTMLSelectElement;
    expect(select.value).toBe("overview");
    const options = Array.from(select.options);
    expect(options).toHaveLength(9);
    const filesOption = options.find((option) => option.value === "files");
    expect(filesOption?.disabled).toBe(true);
    expect(filesOption?.textContent).toContain("Unavailable");
    const overviewOption = options.find((option) => option.value === "overview");
    expect(overviewOption?.disabled).toBe(false);
    // The compact fallback has no `role="tab"` elements at all — it is a real alternative form, not
    // the readable strip hidden behind different markup.
    expect(screen.queryByRole("tab")).toBeNull();
  });

  it("selects a facet by changing the compact select", () => {
    render(<ControlledNav compact={true} />);
    fireEvent.change(screen.getByRole("combobox", { name: "Run detail sections" }), { target: { value: "usage" } });
    expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("usage");
  });

  it("loads and translates the section-nav label in every locale, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      expect(t("missionControl.sectionNav.label")).not.toBe("missionControl.sectionNav.label");
    }
    await activateAppLanguage("en");
  });
});

describe("MissionControlSectionNav", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("defaults to the readable tablist and forwards its container ref, since jsdom fires no ResizeObserver", () => {
    // src/test/setup.ts stubs a global no-op ResizeObserver so this mounts at all; the callback
    // never fires, so `compact` stays at its initial `false` -- the compact-mode *switch* itself is
    // covered directly against MissionControlSectionNavView above, the same way DataTableBody's own
    // compact prop is tested independently of DataTable's own ResizeObserver wiring.
    render(<ControlledNav />);
    expect(screen.getByRole("tablist", { name: "Run detail sections" })).toBeTruthy();
    expect(screen.getByTestId("mission-control-section-nav")).toBeTruthy();
  });
});
