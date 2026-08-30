import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../i18n/format";
import { agentService } from "../services/runtime-agent-client";
import type {
  EvolutionNotificationEvent,
  EvolutionNotificationEventKind,
  SkillEvolutionOrchestrationService,
} from "../services/skill-evolution-orchestration-service";
import { useNotifications } from "./notification-provider";
import type { NotificationType } from "./notification-types";

const types: Record<EvolutionNotificationEventKind, NotificationType> = {
  run_attention: "warning",
  automatic_application: "success",
  probation_regression: "warning",
  breaker_opened: "error",
  breaker_recovered: "info",
};

export function evolutionNotificationPath(event: EvolutionNotificationEvent) {
  const params = new URLSearchParams({
    section: "skills",
    skillWorkspace: "orchestration",
    workspace: event.workspaceId,
  });
  if (event.runId) params.set("evolutionRun", event.runId);
  if (event.applicationId) params.set("evolutionApplication", event.applicationId);
  if (event.probationId) params.set("evolutionProbation", event.probationId);
  if (event.breakerId) params.set("evolutionBreaker", event.breakerId);
  return `/settings?${params.toString()}`;
}

export function EvolutionNotificationBridge({
  service = agentService,
}: {
  service?: Pick<SkillEvolutionOrchestrationService, "subscribeEvolutionNotifications">;
}) {
  const { notify } = useNotifications();
  const { i18n, t } = useTranslation();
  const delivered = useRef(new Set<string>());
  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void service.subscribeEvolutionNotifications((event) => {
      if (!active || delivered.current.has(event.eventId)) return;
      delivered.current.add(event.eventId);
      notify({
        type: types[event.eventKind],
        title: t(`notifications.evolution.${event.eventKind}.title`),
        message: t(`notifications.evolution.${event.eventKind}.message`, {
          skillId: event.skillId ?? t("notifications.evolution.workspaceScope"),
          reason: event.safeReasonCode ?? t("notifications.evolution.safeReasonUnavailable"),
          applicationId: event.applicationId ?? t("notifications.evolution.safeReasonUnavailable"),
          probationEnd: event.probationEndsAtMs === null
            ? t("notifications.evolution.safeReasonUnavailable")
            : formatAppDateTime(event.probationEndsAtMs, i18n.language, {
              dateStyle: "medium",
              timeStyle: "short",
            }),
        }),
        navigation: {
          label: t("notifications.evolution.open"),
          path: evolutionNotificationPath(event),
        },
      });
    }).then((cleanup) => {
      if (active) unsubscribe = cleanup;
      else cleanup();
    }).catch(() => {
      // Native delivery failures are diagnosed at the service boundary.
    });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [i18n.language, notify, service, t]);
  return null;
}
