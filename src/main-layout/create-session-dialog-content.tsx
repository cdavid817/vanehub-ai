import { ChevronLeft, ChevronRight, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";
import { useMediaQuery } from "../hooks/use-media-query";
import { Sheet } from "../ui/sheet/Sheet";
import { CreateSessionStep1 } from "./create-session-step-1";
import { CreateSessionStep2 } from "./create-session-step-2";
import { CreateSessionStep3 } from "./create-session-step-3";
import { CreateSessionStep4 } from "./create-session-step-4";
import { useCreateSessionWizardSteps, CREATE_SESSION_WIZARD_STEP_COUNT } from "./create-session-wizard-steps";
import type { useCreateSessionDraft } from "./use-create-session-draft";
import type { CreateSessionValidation } from "./create-session-validation";

/** Whether Next may advance past `step`, given the draft's current per-field validation (task
 *  11.1's own granularity) — a working default gate; task 11.10 owns where the underlying reasons
 *  actually get displayed, not just whether they block advancing. */
function canAdvancePastStep(step: 1 | 2 | 3 | 4, validation: CreateSessionValidation): boolean {
  if (step === 2) return validation.agent === null && validation.seats === null;
  if (step === 3) return validation.workspace === null && validation.sshConnection === null;
  return true;
}

export function CreateSessionDialogContent({
  model,
  onClose,
  onConfigureOnePiece,
}: {
  model: ReturnType<typeof useCreateSessionDraft>;
  onClose: () => void;
  onConfigureOnePiece: () => void;
}) {
  const { t } = useTranslation();
  const wizard = useCreateSessionWizardSteps();
  const { actions, availableAgents, draft, effectivePersonalizationMode, gitCapable, hasWorkspace, lifecycle, referenceData, selectedAgent, validation } = model;
  const canAdvance = canAdvancePastStep(wizard.step, validation);
  // Task 11.12: `sm:` (640px) is the same breakpoint `ApplicationDialog` already treats
  // specially in its own padding (`p-3 sm:p-5`) -- below it, a centered `max-w-2xl` dialog leaves
  // too little room around a multi-field form, so the wizard becomes a full-height `Sheet`
  // instead, sharing every other prop (title/description/footer/onClose/closeDisabled) verbatim.
  const wide = useMediaQuery("(min-width: 640px)");

  const footer = (
    <div className="flex items-start justify-between gap-3">
      <p className="min-w-0 flex-1 wrap-break-word text-xs leading-5 text-destructive" role="alert">
        {lifecycle.error}
      </p>
      <div className="flex shrink-0 items-center gap-2">
        {wizard.isFirstStep ? (
          <Button className="h-8 px-3 text-xs" disabled={lifecycle.loading} onClick={onClose} type="button" variant="outline">
            {t("createSession.cancel")}
          </Button>
        ) : (
          <Button className="h-8 px-3 text-xs" disabled={lifecycle.loading} onClick={wizard.goBack} type="button" variant="outline">
            <ChevronLeft className="h-3.5 w-3.5" aria-hidden="true" />
            {t("createSession.back")}
          </Button>
        )}
        {wizard.isLastStep ? (
          <Button className="h-8 px-3 text-xs" disabled={!validation.canSubmit || lifecycle.loading} onClick={actions.submit} type="button">
            {lifecycle.loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : null}
            {t("createSession.create")}
          </Button>
        ) : (
          <Button className="h-8 px-3 text-xs" disabled={!canAdvance} onClick={wizard.goNext} type="button">
            {t("createSession.next")}
            <ChevronRight className="h-3.5 w-3.5" aria-hidden="true" />
          </Button>
        )}
      </div>
    </div>
  );

  const steps = (
      <div className="grid gap-4">
        {wizard.step === 1 ? (
          <CreateSessionStep1
            agentMode={draft.agentMode}
            availableAgents={availableAgents}
            interactionMode={draft.interactionMode}
            onAgentModeChange={actions.setAgentMode}
            onInteractionModeChange={actions.setInteractionMode}
            onWorkspaceModeChange={actions.setWorkspaceMode}
            selectedAgent={selectedAgent}
            workspaceMode={draft.workspaceMode}
          />
        ) : null}
        {wizard.step === 2 ? (
          <CreateSessionStep2
            agentMode={draft.agentMode}
            availableAgents={availableAgents}
            expertRoles={referenceData.expertRoles}
            hasWorkspace={hasWorkspace}
            multiSeats={draft.multiSeats}
            onAgentSelect={actions.selectAgent}
            onConfigureOnePiece={onConfigureOnePiece}
            onPersonalizationModeChange={actions.setPersonalizationMode}
            onSeatsChange={actions.setSeats}
            personalizationMode={effectivePersonalizationMode}
            selectedAgent={selectedAgent}
            validation={validation}
          />
        ) : null}
        {wizard.step === 3 ? (
          <CreateSessionStep3
            gitCapable={gitCapable}
            inspection={referenceData.inspection}
            knownProjects={referenceData.knownProjects}
            knownRemoteWorkspaces={referenceData.knownRemoteWorkspaces}
            onBrowseProject={actions.browseProject}
            onInspectPath={actions.inspectPath}
            projectPath={draft.projectPath}
            remoteDisplayName={draft.remoteDisplayName}
            remoteHost={draft.remoteHost}
            remotePath={draft.remotePath}
            remotePort={draft.remotePort}
            remoteUser={draft.remoteUser}
            saveSshConnection={draft.saveSshConnection}
            selectedSshConnectionId={draft.selectedSshConnectionId}
            setProjectPath={actions.setProjectPath}
            setRemoteDisplayName={actions.setRemoteDisplayName}
            setRemoteHost={actions.setRemoteHost}
            setRemotePath={actions.setRemotePath}
            setRemotePort={actions.setRemotePort}
            setRemoteUser={actions.setRemoteUser}
            setSaveSshConnection={actions.setSaveSshConnection}
            setSelectedSshConnectionId={actions.setSelectedSshConnectionId}
            setSshConnectionDraft={actions.setSshConnectionDraft}
            setWorktreeEnabled={actions.setWorktreeEnabled}
            setWorktreeName={actions.setWorktreeName}
            sshConnectionDraft={draft.sshConnectionDraft}
            sshConnections={referenceData.sshConnections}
            validation={validation}
            workspaceMode={draft.workspaceMode}
            worktreeEnabled={draft.worktreeEnabled}
            worktreeName={draft.worktreeName}
          />
        ) : null}
        {wizard.step === 4 ? (
          <CreateSessionStep4
            draft={draft}
            effectivePersonalizationMode={effectivePersonalizationMode}
            onGoToStep={wizard.goToStep}
            onTitleChange={actions.setTitle}
            selectedAgent={selectedAgent}
            validation={validation}
          />
        ) : null}
      </div>
  );

  const description = t("createSession.step", { current: wizard.step, total: CREATE_SESSION_WIZARD_STEP_COUNT });
  const title = t("createSession.title");

  return wide ? (
    <ApplicationDialog closeDisabled={lifecycle.loading} description={description} footer={footer} onClose={onClose} title={title}>
      {steps}
    </ApplicationDialog>
  ) : (
    <Sheet closeDisabled={lifecycle.loading} description={description} footer={footer} onClose={onClose} placement="full" title={title}>
      {steps}
    </Sheet>
  );
}
