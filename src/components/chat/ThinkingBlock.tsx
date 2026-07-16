import { BrainCircuit } from "lucide-react";
import { useTranslation } from "react-i18next";

export function ThinkingBlock({ content }: { content: string }) {
  const { t } = useTranslation();
  if (!content.trim()) return null;
  return (
    <details className="mt-3 rounded-md border border-border bg-muted/60 text-xs">
      <summary className="flex cursor-pointer items-center gap-2 px-3 py-2 text-muted-foreground">
        <BrainCircuit className="h-3.5 w-3.5" aria-hidden="true" />
        {t("chat.thinking")}
      </summary>
      <div className="whitespace-pre-wrap border-t border-border px-3 py-2 leading-5 text-muted-foreground">
        {content}
      </div>
    </details>
  );
}
