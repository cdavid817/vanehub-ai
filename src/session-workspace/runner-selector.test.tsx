// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { AgentRunnerDescriptor } from "../types/agent-runner";
import { RunnerSelector } from "./runner-selector";

const descriptors: AgentRunnerDescriptor[] = [
  { selection: { kind: "local" }, label: "Local", hostLabel: "This device", available: true, unavailableReason: null, simulated: false, capabilities: { interactiveInput: true, pty: false, cancellation: true, inspection: true, recovery: "none" } },
  { selection: { kind: "ssh", targetId: "ssh-1", targetRevision: 3 }, label: "Build host", hostLabel: "bounded.example.test", available: true, unavailableReason: null, simulated: false, capabilities: { interactiveInput: true, pty: true, cancellation: true, inspection: true, recovery: "inspect_only" } },
  { selection: { kind: "docker" }, label: "Docker / Sandbox", hostLabel: null, available: false, unavailableReason: "runner_not_implemented", simulated: false, capabilities: { interactiveInput: false, pty: false, cancellation: false, inspection: false, recovery: "none" } },
];

describe("RunnerSelector", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("supports keyboard selection and exposes unavailable choices without enabling them", () => {
    const onChange = vi.fn();
    renderWithAppProviders(<RunnerSelector descriptors={descriptors} disabled={false} error={false} loading={false} onChange={onChange} onRetry={vi.fn()} value={{ kind: "local" }} />);
    const select = screen.getByLabelText("Runner") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "1" } });
    expect(onChange).toHaveBeenCalledWith({ kind: "ssh", targetId: "ssh-1", targetRevision: 3 });
    expect((screen.getByRole("option", { name: /Docker.*Not implemented/ }) as HTMLOptionElement).disabled).toBe(true);
    expect(screen.getByTestId("runner-selector").className).toContain("min-w-0");
  });

  it("renders loading, error, retry, and disabled states accessibly", () => {
    const retry = vi.fn();
    const { rerender } = renderWithAppProviders(<RunnerSelector descriptors={[]} disabled={false} error={false} loading onChange={vi.fn()} onRetry={retry} value={{ kind: "local" }} />);
    expect((screen.getByLabelText("Runner") as HTMLSelectElement).disabled).toBe(true);
    rerender(<RunnerSelector descriptors={[]} disabled={false} error loading={false} onChange={vi.fn()} onRetry={retry} value={{ kind: "local" }} />);
    fireEvent.click(screen.getByRole("button", { name: "Retry Runner discovery" }));
    expect(retry).toHaveBeenCalledOnce();
  });
});
