import { useState } from "react";

export const EVALUATION_WIZARD_STEP_COUNT = 3;

export type EvaluationWizardStep = 1 | 2 | 3;

export interface EvaluationWizardSteps {
  step: EvaluationWizardStep;
  isFirstStep: boolean;
  isLastStep: boolean;
  goNext: () => void;
  goBack: () => void;
  /** For the Review step (18.4) to jump straight back to the step that owns a field. */
  goToStep: (step: EvaluationWizardStep) => void;
}

/**
 * Step-navigation state for the evaluation run wizard (18.4): task, then Agent selection, then
 * Review. Same shape as `useCreateSessionWizardSteps` -- this codebase's one existing multi-step
 * wizard shell -- but defined locally rather than shared: `useCreateSessionWizardSteps` is typed
 * to its own 4-step `CreateSessionWizardStep` literal and lives beside session-creation's own
 * draft model, with no generic wizard-shell primitive extracted anywhere under `src/ui/`.
 *
 * `EvaluationRunWizard` remounts fresh every time it opens (its parent, `EvaluationRunControls`,
 * only mounts it while the Sheet is showing -- same pattern `CreateSessionDialogContent` itself
 * relies on), so a plain `useState(1)` already starts back at step 1 on every open with no extra
 * reset effect needed.
 */
export function useEvaluationWizardSteps(): EvaluationWizardSteps {
  const [step, setStep] = useState<EvaluationWizardStep>(1);

  function goNext() {
    setStep((current) => (current === EVALUATION_WIZARD_STEP_COUNT ? current : (current + 1) as EvaluationWizardStep));
  }

  function goBack() {
    setStep((current) => (current === 1 ? current : (current - 1) as EvaluationWizardStep));
  }

  return { step, isFirstStep: step === 1, isLastStep: step === EVALUATION_WIZARD_STEP_COUNT, goNext, goBack, goToStep: setStep };
}
