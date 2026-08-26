import { FileText, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { PersonalizationPolicyRef } from "../../../types/personalization";
import { SectionPanel } from "../page-parts";
import { InstructionEditor } from "./instruction-editor";
import { isIncomplete, PersonalizationScopeSelector } from "./scope-selector";
import { useInstructionDrafts } from "./use-instruction-drafts";
import { useScopeOptions } from "./use-scope-options";

/**
 * The Instructions destination: which layer, then that layer's text.
 *
 * The two panels stay separate because they fail separately -- an unreadable layer still leaves the
 * selector usable, and an incomplete selection has no text to show at all.
 */
export function PersonalizationInstructionsView({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [scope, setScope] = useState<PersonalizationPolicyRef>({ scopeKind: "global" });
  const { agents, workspaces } = useScopeOptions(service);
  const drafts = useInstructionDrafts(service, scope);
  const incomplete = isIncomplete(scope);

  return (
    <div className="grid gap-5">
      <SectionPanel
        description={t("personalization.scope.description")}
        icon={SlidersHorizontal}
        title={t("personalization.scope.title")}
      >
        <PersonalizationScopeSelector
          agents={agents}
          onChange={setScope}
          scope={scope}
          workspaces={workspaces}
        />
        <p className="mt-4 text-sm text-muted-foreground" data-testid="personalization-scope-status">
          {scopeStatus()}
        </p>
      </SectionPanel>

      <SectionPanel
        description={t("personalization.editor.description")}
        icon={FileText}
        title={t("personalization.editor.title")}
      >
        {editorBody()}
      </SectionPanel>
    </div>
  );

  function scopeStatus(): string {
    if (incomplete) return t("personalization.scope.status.incomplete");
    if (drafts.isLoading) return t("personalization.scope.status.loading");
    if (drafts.loadError) return t("personalization.scope.status.unavailable");
    // A layer that has never been written is not the same as one written to all-inherit: the first
    // has no revision to conflict against, and saying so is what makes the next save legible.
    if (!drafts.draft || drafts.draft.baseRevision === 0) {
      return t("personalization.scope.status.neverWritten");
    }
    return t("personalization.scope.status.written", { revision: drafts.draft.baseRevision });
  }

  function editorBody() {
    if (incomplete) {
      return (
        <p className="text-sm text-muted-foreground" data-testid="personalization-editor-unselected">
          {t("personalization.scope.incomplete")}
        </p>
      );
    }
    if (drafts.loadError) {
      return (
        <p className="text-sm ucd-status-danger" data-testid="personalization-editor-error" role="alert">
          {t("personalization.editor.loadFailed")}
        </p>
      );
    }
    if (!drafts.draft) {
      return (
        <p className="text-sm text-muted-foreground" data-testid="personalization-editor-loading">
          {t("personalization.scope.status.loading")}
        </p>
      );
    }
    return (
      <InstructionEditor
        draft={drafts.draft}
        onDiscard={drafts.discard}
        onEdit={drafts.edit}
        onSave={drafts.save}
      />
    );
  }
}
