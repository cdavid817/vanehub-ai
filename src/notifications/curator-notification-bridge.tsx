import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { SkillCuratorService } from "../services/skill-curator-service";
import { agentService } from "../services/runtime-agent-client";
import type { CuratorNotificationEvent, CuratorNotificationEventKind } from "../types/skill-curator";
import { useNotifications } from "./notification-provider";
import type { NotificationType } from "./notification-types";

const types: Record<CuratorNotificationEventKind, NotificationType> = {
  pending_review: "info",
  deferral_date: "info",
  supersession: "warning",
  rejection: "warning",
  apply_success: "success",
  apply_failure: "error",
  probation_regression: "warning",
};

const keys: Record<CuratorNotificationEventKind, string> = {
  pending_review: "pendingReview",
  deferral_date: "deferralDate",
  supersession: "supersession",
  rejection: "rejection",
  apply_success: "applySuccess",
  apply_failure: "applyFailure",
  probation_regression: "probationRegression",
};

export function curatorNotificationPath(event: CuratorNotificationEvent): string {
  const params = new URLSearchParams({
    section: "skills",
    skillWorkspace: "curator",
    candidate: event.candidateId,
    workspace: event.workspaceId,
  });
  if (event.navigationTarget.kind === "overlay_history") {
    params.set("skillWorkspace", "inventory");
    params.set("overlayHistory", event.navigationTarget.overlayHistoryId);
    params.set("skill", event.navigationTarget.skillId);
  }
  return `/settings?${params.toString()}`;
}

export function CuratorNotificationBridge({
  service = agentService,
}: {
  service?: Pick<SkillCuratorService, "subscribeSkillCuratorNotifications">;
}) {
  const { notify } = useNotifications();
  const { t } = useTranslation();

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void service.subscribeSkillCuratorNotifications((event) => {
      if (!active) return;
      const key = keys[event.eventKind];
      const history = event.navigationTarget.kind === "overlay_history";
      notify({
        type: types[event.eventKind],
        title: t(`notifications.curator.${key}.title`),
        message: t(`notifications.curator.${key}.message`, {
          skillId: event.skillId,
          risk: event.risk,
          route: event.route,
          scope: event.overlayScope,
        }),
        navigation: {
          label: t(history ? "notifications.curator.openHistory" : "notifications.curator.openCandidate"),
          path: curatorNotificationPath(event),
        },
      });
    }).then((cleanup) => {
      if (active) unsubscribe = cleanup;
      else cleanup();
    }).catch(() => {
      // Native delivery diagnostics are persisted by the service boundary.
    });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [notify, service, t]);

  return null;
}
