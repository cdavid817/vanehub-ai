import { EvaluationCenter } from "../../evaluation-center/evaluation-center";

/**
 * The evaluation center hosted as a settings page. It is a diagnostic tool rather than a daily
 * destination, so it lives beside the other Agent settings; the center itself is unchanged.
 */
export function EvaluationPage() {
  return (
    <div className="flex min-h-[70vh] flex-col" data-testid="evaluation-settings-page">
      <EvaluationCenter />
    </div>
  );
}
