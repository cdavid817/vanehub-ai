import { useEffect, useId, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

export function MermaidDiagram({ chart }: { chart: string }) {
  const { t } = useTranslation();
  const id = useId().replace(/[^a-zA-Z0-9_-]/g, "");
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setSvg(null);
    setError(null);
    void import("mermaid")
      .then(async ({ default: mermaid }) => {
        mermaid.initialize({ startOnLoad: false, securityLevel: "strict", theme: "neutral" });
        const rendered = await mermaid.render(`mermaid-${id}`, chart);
        if (!cancelled) setSvg(rendered.svg);
      })
      .catch((reason: unknown) => {
        if (cancelled) return;
        setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => {
      cancelled = true;
    };
  }, [chart, id]);

  if (error) {
    return (
      <div className="my-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive">
        <AlertTriangle className="mr-1 inline h-3.5 w-3.5" aria-hidden="true" />
        {t("chat.mermaidFailed")}
      </div>
    );
  }

  if (!svg) return <div className="my-2 h-20 animate-pulse rounded-md bg-muted" aria-label={t("chat.mermaidLoading")} />;

  return <div className="my-2 overflow-x-auto rounded-md border border-border bg-background p-3" dangerouslySetInnerHTML={{ __html: svg }} />;
}
