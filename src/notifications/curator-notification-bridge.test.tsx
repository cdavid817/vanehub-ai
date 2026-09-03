// @vitest-environment jsdom

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SkillCuratorService } from "../services/skill-curator-service";
import type { CuratorNotificationEvent } from "../types/skill-curator";
import "../i18n";
import { NotificationCenter } from "./notification-center";
import { CuratorNotificationBridge, curatorNotificationPath } from "./curator-notification-bridge";
import { NotificationProvider } from "./notification-provider";

const event: CuratorNotificationEvent = {
  schemaVersion: 1,
  eventKind: "pending_review",
  candidateId: "candidate/1",
  candidateRevision: 2,
  workspaceId: "/safe workspace",
  skillId: "review",
  overlayScope: "project",
  state: "ready_for_review",
  risk: "medium",
  route: "needs_human_review",
  navigationTarget: { kind: "candidate_review", candidateId: "candidate/1" },
};

describe("Curator notification bridge", () => {
  it("creates an encoded settings navigation target", () => {
    const path = curatorNotificationPath(event);
    expect(path).toContain("section=skills");
    expect(path).toContain("candidate=candidate%2F1");
    expect(path).toContain("workspace=%2Fsafe+workspace");
  });

  it("turns a notification action into navigation only", async () => {
    let handler: ((value: CuratorNotificationEvent) => void) | undefined;
    const subscribe = vi.fn(async (next: (value: CuratorNotificationEvent) => void) => {
      handler = next;
      return vi.fn();
    });
    const navigate = vi.fn();
    const view = render(
      <NotificationProvider onNavigate={navigate}>
        <CuratorNotificationBridge service={{
          subscribeSkillCuratorNotifications: subscribe,
        } as Pick<SkillCuratorService, "subscribeSkillCuratorNotifications">} />
        <NotificationCenter />
      </NotificationProvider>,
    );
    await waitFor(() => expect(handler).toBeTypeOf("function"));
    act(() => handler?.(event));
    const trigger = view.container.querySelector<HTMLButtonElement>("[aria-controls='notification-center']");
    expect(trigger).not.toBeNull();
    fireEvent.click(trigger!);
    fireEvent.click(await screen.findByRole("button", { name: /Skill 变更待审查/i }));

    expect(navigate).toHaveBeenCalledWith(curatorNotificationPath(event));
    expect(subscribe).toHaveBeenCalledOnce();
  });
});
