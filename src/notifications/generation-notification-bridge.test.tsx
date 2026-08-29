// @vitest-environment jsdom

import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { activateAppLanguage } from "../i18n";
import type { GenerationNotificationEvent, SkillGenerationService } from "../services/skill-generation-service";
import { GenerationNotificationBridge, generationNotificationPath } from "./generation-notification-bridge";
import { NotificationProvider, useNotifications } from "./notification-provider";

const event: GenerationNotificationEvent = {
  schemaVersion: 1,
  eventId: "review-ready:job/1:1",
  eventKind: "review_ready",
  jobId: "job/1",
  workspaceId: "/safe workspace",
  seedId: "seed-1",
};

function Count() {
  const { notifications } = useNotifications();
  return <output aria-label="notification count">{notifications.length}</output>;
}

describe("Generation notification bridge", () => {
  beforeEach(() => activateAppLanguage("en"));

  it("creates an encoded navigation-only generation target", () => {
    const path = generationNotificationPath(event);
    expect(path).toContain("skillWorkspace=generation");
    expect(path).toContain("generationJob=job%2F1");
    expect(path).toContain("workspace=%2Fsafe+workspace");
  });

  it("deduplicates safe events without exposing mutation actions", async () => {
    let handler: ((value: GenerationNotificationEvent) => void) | undefined;
    const subscribe = vi.fn(async (next: (value: GenerationNotificationEvent) => void) => {
      handler = next;
      return vi.fn();
    });
    render(<NotificationProvider><GenerationNotificationBridge service={{ subscribeGenerationNotifications: subscribe } as Pick<SkillGenerationService, "subscribeGenerationNotifications">} /><Count /></NotificationProvider>);
    await waitFor(() => expect(handler).toBeTypeOf("function"));
    act(() => { handler?.(event); handler?.(event); });
    expect(screen.getByLabelText("notification count").textContent).toBe("1");
    expect(screen.queryByRole("button", { name: /approve|apply|install|regenerate|cancel/i })).toBeNull();
  });

  it("isolates subscription failures", async () => {
    const service = { async subscribeGenerationNotifications() { throw new Error("native event unavailable"); } } as Pick<SkillGenerationService, "subscribeGenerationNotifications">;
    render(<NotificationProvider><GenerationNotificationBridge service={service} /><Count /></NotificationProvider>);
    await waitFor(() => expect(screen.getByLabelText("notification count").textContent).toBe("0"));
  });
});
