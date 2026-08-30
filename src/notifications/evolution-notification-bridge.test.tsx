// @vitest-environment jsdom

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type {
  EvolutionNotificationEvent,
  SkillEvolutionOrchestrationService,
} from "../services/skill-evolution-orchestration-service";
import { EvolutionNotificationBridge, evolutionNotificationPath } from "./evolution-notification-bridge";
import { NotificationCenter } from "./notification-center";
import { NotificationProvider } from "./notification-provider";

const event: EvolutionNotificationEvent = {
  schemaVersion: 1,
  eventId: "breaker_opened:breaker/1:2",
  eventKind: "breaker_opened",
  workspaceId: "workspace safe",
  runId: null,
  applicationId: null,
  probationId: null,
  breakerId: "breaker/1",
  skillId: null,
  safeReasonCode: "integrity_failure",
  probationEndsAtMs: null,
  entityRevision: 2,
};

describe("Skill evolution notification bridge", () => {
  it("deduplicates safe events and provides navigation-only action", async () => {
    let handler: ((value: EvolutionNotificationEvent) => void) | undefined;
    const subscribe = vi.fn(async (next: (value: EvolutionNotificationEvent) => void) => {
      handler = next;
      return vi.fn();
    });
    const navigate = vi.fn();
    const view = render(
      <NotificationProvider onNavigate={navigate}>
        <EvolutionNotificationBridge service={{
          subscribeEvolutionNotifications: subscribe,
        } as Pick<SkillEvolutionOrchestrationService, "subscribeEvolutionNotifications">} />
        <NotificationCenter />
      </NotificationProvider>,
    );
    await waitFor(() => expect(handler).toBeTypeOf("function"));
    act(() => {
      handler?.(event);
      handler?.(event);
    });
    const trigger = view.container.querySelector<HTMLButtonElement>("[aria-controls='notification-center']");
    fireEvent.click(trigger!);
    expect(screen.getAllByText("Skill 自动更新已暂停")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: /Skill 自动更新已暂停/i }));
    expect(navigate).toHaveBeenCalledWith(evolutionNotificationPath(event));
    expect(evolutionNotificationPath(event)).toContain("evolutionBreaker=breaker%2F1");
  });
});
