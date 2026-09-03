// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopRunPhaseOrder, PhaseStepper } from "./phase-stepper";

/** Finds the `<li>` ancestor rendered for a given phase label, so a test can inspect its
 *  complete/active/pending treatment without depending on lucide's internal icon class names. */
function phaseItem(label: string) {
  const item = screen.getByText(label).closest("li");
  if (!item) throw new Error(`no <li> ancestor for "${label}"`);
  return item;
}

describe("PhaseStepper", () => {
  it("renders every phase, in order, under a labelled list", () => {
    render(<PhaseStepper phase="preparing" status="queued" />);
    const list = screen.getByRole("list", { name: "运行阶段" });
    expect([...list.querySelectorAll("li")].map((item) => item.textContent)).toEqual(["准备", "执行", "验证", "决策", "收尾"]);
  });

  it("marks phases before the current one complete, the current one active, and later ones pending", () => {
    render(<PhaseStepper phase="verifying" status="running" />);
    expect(phaseItem("准备").className).toContain("border-primary");
    expect(phaseItem("准备").querySelector(".animate-spin")).toBeNull();
    expect(phaseItem("执行").className).toContain("border-primary");
    expect(phaseItem("执行").querySelector(".animate-spin")).toBeNull();
    expect(phaseItem("验证").className).toContain("border-primary");
    expect(phaseItem("验证").querySelector(".animate-spin")).not.toBeNull();
    expect(phaseItem("决策").className).toContain("border-border");
    expect(phaseItem("收尾").className).toContain("border-border");
  });

  it("marks every phase complete, and none active, once the run reaches a terminal state", () => {
    render(<PhaseStepper phase="finalizing" status="succeeded" />);
    for (const label of ["准备", "执行", "验证", "决策", "收尾"]) {
      expect(phaseItem(label).className).toContain("border-primary");
      expect(phaseItem(label).querySelector(".animate-spin")).toBeNull();
    }
  });

  it("marks every phase complete while awaiting acceptance, even ones after the reported phase", () => {
    render(<PhaseStepper phase="finalizing" status="awaiting-acceptance" />);
    expect(phaseItem("准备").className).toContain("border-primary");
    expect(phaseItem("收尾").className).toContain("border-primary");
  });

  it("accepts a caller-supplied phase order instead of the default five", () => {
    render(<PhaseStepper phase="acting" phases={["preparing", "acting"]} status="running" />);
    const list = screen.getByRole("list", { name: "运行阶段" });
    expect([...list.querySelectorAll("li")].map((item) => item.textContent)).toEqual(["准备", "执行"]);
  });

  it("exports the canonical five-phase run order", () => {
    expect(loopRunPhaseOrder).toEqual(["preparing", "acting", "verifying", "deciding", "finalizing"]);
  });
});
