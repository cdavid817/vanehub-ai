import { useTranslation } from "react-i18next";
import { FormSection } from "../ui/forms/FormSection";
import { LoopVerificationCommandEditor } from "./loop-verification-command-editor";
import { NumberField, type StepProps } from "./loop-definition-step-fields";

export function VerificationStep({ draft, setDraft, showErrors }: StepProps & { showErrors: boolean }) {
  const { t } = useTranslation();
  return (
    <FormSection title={t("loops.editor.step.verification")}>
      <div className="grid gap-5">
        <LoopVerificationCommandEditor commands={draft.verificationCommands} onChange={(verificationCommands) => setDraft({ ...draft, verificationCommands })} showErrors={showErrors} />
        <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          <NumberField field="maxIterations" max={20} draft={draft} setDraft={setDraft} />
          <NumberField field="stepTimeoutSeconds" draft={draft} setDraft={setDraft} />
          <NumberField field="totalTimeoutSeconds" draft={draft} setDraft={setDraft} />
          <NumberField field="maxConsecutiveRuntimeErrors" draft={draft} setDraft={setDraft} />
          <NumberField field="maxConsecutiveNoProgress" draft={draft} setDraft={setDraft} />
        </section>
      </div>
    </FormSection>
  );
}
