import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { seatsFromSession } from "../services/session-seats";
import type { Session } from "../types/agent";
import { Accordion, type AccordionItem } from "../ui/accordion/Accordion";
import { EvidenceLink } from "../ui/evidence/EvidenceLink";
import type { InspectorProviderContext } from "../ui/inspector/inspector-provider-registry";
import { SessionCodeIndexPane } from "./session-code-index-pane";
import { SessionImPane } from "./session-im-pane";
import { SessionOverviewRuntimeSection, SessionOverviewWorkspaceSection } from "./session-overview-runtime-workspace";
import { SessionRosterEditor } from "./session-roster-editor";
import { SessionSkillsPane } from "./session-skills-pane";
import { SessionTokenUsagePane } from "./session-token-usage-pane";

/** Runtime and Workspace are the two halves split out of the old always-visible Basic Info tab. */
const DEFAULT_OPEN_IDS = ["runtime", "workspace"];

export interface SessionOverviewSectionsProps {
  session: Session;
  context: InspectorProviderContext;
}

/**
 * The migrated replacement for session-info-panel.tsx's six-tab `role="tablist"` (design.md task
 * 9.15's named anti-pattern: "four to six equal-width text tabs in a 300px panel") — seven
 * independent Accordion sections instead. Participants and Code Index render only under the same
 * conditions the old panel used, so nothing that used to appear starts appearing unconditionally.
 */
export function SessionOverviewSections({ session, context }: SessionOverviewSectionsProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [openIds, setOpenIds] = useState<string[]>(DEFAULT_OPEN_IDS);

  // Coordinator-supplied, e.g. the conversation header's "Open IM" button or inspecting a loop
  // iteration's usage target — mirrors the pre-migration information panel's own `requestedTab`
  // (a value that is set but never reset to null, so this only needs to react to genuine changes,
  // not force itself open on every unrelated render).
  useEffect(() => {
    const requested = context.requestedSessionSection;
    if (!requested) return;
    setOpenIds((ids) => (ids.includes(requested) ? ids : [...ids, requested]));
  }, [context.requestedSessionSection]);

  const showParticipants = seatsFromSession(session).length > 1;
  // Same two-path distinction session-info-panel.tsx makes: `workspacePath` (no `folder` fallback)
  // is the exact condition the old panel used to decide Code Index, kept unchanged here;
  // `workspaceDisplayPath` additionally falls back to `folder` because it is what a reader sees,
  // not something anything branches on.
  const workspacePath = session.worktreePath ?? session.projectPath ?? null;
  const workspaceDisplayPath = workspacePath ?? session.folder ?? null;
  const showCodeIndex = session.agentId === "onepiece" && Boolean(workspacePath);

  function isOpen(id: string): boolean {
    return openIds.includes(id);
  }

  // Local Accordion state only — Usage is a section here, not a route or a workspace tab, so
  // nothing outside this component needs to know it happened.
  function showUsageSection() {
    setOpenIds((ids) => (ids.includes("usage") ? ids : [...ids, "usage"]));
  }

  const items: AccordionItem[] = [
    ...(showParticipants
      ? [{
          id: "participants",
          // Reuses the same label session-info-panel.tsx already gave this tab — the pane
          // rendered beneath it (unchanged) repeats it as its own heading, exactly as it does
          // in the panel being migrated.
          header: t("session.memberInfo"),
          // Live turn state (who is currently speaking) is not derivable from `session` alone —
          // it comes through `context`, the same way the pre-migration information panel got it
          // from main-layout.tsx's already-loaded chat state.
          content: (
            <SessionRosterEditor
              currentSpeakerSeatId={context.currentSpeakerSeatId ?? null}
              messages={context.messages ?? []}
              session={session}
            />
          ),
        }]
      : []),
    {
      id: "runtime",
      header: t("sessionOverview.section.runtime"),
      content: <SessionOverviewRuntimeSection session={session} />,
    },
    {
      id: "workspace",
      header: t("layout.info.workspace"),
      content: (
        <SessionOverviewWorkspaceSection
          active={isOpen("workspace")}
          context={context}
          displayPath={workspaceDisplayPath}
          onShowUsage={showUsageSection}
          session={session}
        />
      ),
    },
    {
      id: "usage",
      header: t("layout.infoTab.tokenUsage"),
      content: <SessionTokenUsagePane active={isOpen("usage")} lifecycle={session.lifecycleState} sessionId={session.id} />,
    },
    {
      id: "skills",
      header: t("layout.infoTab.skills"),
      content: (
        <div className="grid gap-3">
          {/* Settings is a real route, unlike workspace tabs — an EvidenceLink alongside the
              pane's own existing affordance, not instead of it (task 9.9). */}
          <EvidenceLink availability="available" label={t("sessionOverview.openSkillSettings")} to="/settings?section=skills" />
          <SessionSkillsPane active={isOpen("skills")} activeSession={session} onOpenSkillSettings={() => navigate("/settings?section=skills")} />
        </div>
      ),
    },
    {
      id: "im",
      header: t("layout.infoTab.im"),
      content: (
        <div className="grid gap-3">
          <EvidenceLink availability="available" label={t("sessionOverview.openImSettings")} to="/settings?section=im" />
          <SessionImPane active={isOpen("im")} onOpenSettings={() => navigate("/settings?section=im")} sessionId={session.id} />
        </div>
      ),
    },
    ...(showCodeIndex && workspacePath
      ? [{
          id: "code-index",
          header: t("layout.infoTab.codeIndex"),
          content: <SessionCodeIndexPane active={isOpen("code-index")} workspacePath={workspacePath} />,
        }]
      : []),
  ];

  return <Accordion items={items} onOpenIdsChange={setOpenIds} openIds={openIds} />;
}
