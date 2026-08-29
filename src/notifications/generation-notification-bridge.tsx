import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { GenerationNotificationEvent, SkillGenerationService } from "../services/skill-generation-service";
import { useNotifications } from "./notification-provider";
import type { NotificationType } from "./notification-types";

const types: Record<GenerationNotificationEvent["eventKind"], NotificationType> = {
  review_ready: "success",
  attention_required: "error",
  cancelled: "info",
  superseded: "warning",
};

export function generationNotificationPath(event: GenerationNotificationEvent): string {
  return `/settings?${new URLSearchParams({
    section: "skills",
    skillWorkspace: "generation",
    workspace: event.workspaceId,
    generationJob: event.jobId,
  }).toString()}`;
}

export function GenerationNotificationBridge({
  service = agentService,
}: {
  service?: Pick<SkillGenerationService, "subscribeGenerationNotifications">;
}) {
  const { notify } = useNotifications();
  const { t } = useTranslation();
  const delivered = useRef(new Set<string>());
  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void service.subscribeGenerationNotifications((event) => {
      if (!active || delivered.current.has(event.eventId)) return;
      delivered.current.add(event.eventId);
      notify({
        type: types[event.eventKind],
        title: t(`notifications.generation.${event.eventKind}.title`),
        message: t(`notifications.generation.${event.eventKind}.message`, {
          seedId: event.seedId,
          reason: event.safeFailureCode ?? t("notifications.generation.safeReasonUnavailable"),
        }),
        navigation: {
          label: t("notifications.generation.openJob"),
          path: generationNotificationPath(event),
        },
      });
    }).then((cleanup) => { if (active) unsubscribe = cleanup; else cleanup(); }).catch(() => {
      // Subscription failures are diagnosed by the native service and never block Agent work.
    });
    return () => { active = false; unsubscribe?.(); };
  }, [notify, service, t]);
  return null;
}
