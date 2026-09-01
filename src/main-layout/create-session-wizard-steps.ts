import { useState } from "react";

export const CREATE_SESSION_WIZARD_STEP_COUNT = 4;

export type CreateSessionWizardStep = 1 | 2 | 3 | 4;

export interface CreateSessionWizardSteps {
  step: CreateSessionWizardStep;
  isFirstStep: boolean;
  isLastStep: boolean;
  goNext: () => void;
  goBack: () => void;
  /** For the Review step (11.7) to jump straight back to the step that owns a field, and for a
   *  validation-error summary (11.10, not yet built) to link to the owning step directly. */
  goToStep: (step: CreateSessionWizardStep) => void;
}

/**
 * Step-navigation state for the create-session wizard (task 11.3-11.8), deliberately independent
 * of `useCreateSessionDraft` (task 11.1's own model): which step is showing is presentation
 * state, not draft content, and `CreateSessionDialogContent` already remounts fresh every time
 * the dialog opens (`CreateSessionDialog.tsx`'s own `if (!open) return null;` unmounts it
 * entirely), so a plain `useState(1)` here already starts back at step 1 on every open with no
 * extra reset effect needed.
 */
export function useCreateSessionWizardSteps(): CreateSessionWizardSteps {
  const [step, setStep] = useState<CreateSessionWizardStep>(1);

  function goNext() {
    setStep((current) => (current === CREATE_SESSION_WIZARD_STEP_COUNT ? current : (current + 1) as CreateSessionWizardStep));
  }

  function goBack() {
    setStep((current) => (current === 1 ? current : (current - 1) as CreateSessionWizardStep));
  }

  return {
    step,
    isFirstStep: step === 1,
    isLastStep: step === CREATE_SESSION_WIZARD_STEP_COUNT,
    goNext,
    goBack,
    goToStep: setStep,
  };
}
