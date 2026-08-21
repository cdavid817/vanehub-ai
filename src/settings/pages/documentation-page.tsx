import { BookOpen, ExternalLink } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import englishReadme from "../../../README.md?raw";
import japaneseReadme from "../../../README.ja.md?raw";
import simplifiedChineseReadme from "../../../README.zh-CN.md?raw";
import { RichMarkdown } from "../../components/chat/RichMarkdown";
import { Button } from "../../components/ui/button";
import { aboutRepositoryUrl } from "../../services/about-service";
import { PageHeader, SectionPanel } from "./page-parts";

/**
 * Bundled at build time rather than fetched: the desktop client has to answer "what is this and
 * how do I use it" with no network, and the README that ships with a build is the one that
 * describes that build.
 */
const readmeByLanguage: Record<string, string> = {
  ja: japaneseReadme,
  zh: simplifiedChineseReadme,
};

export function resolveReadme(language: string): string {
  const base = language.toLowerCase().split("-")[0];
  return readmeByLanguage[base] ?? englishReadme;
}

export function DocumentationPage() {
  const { i18n, t } = useTranslation();
  const readme = useMemo(() => resolveReadme(i18n.language), [i18n.language]);

  return (
    <div className="grid gap-6">
      <PageHeader
        actions={(
          <Button
            onClick={() => window.open(aboutRepositoryUrl, "_blank", "noreferrer")}
            size="sm"
            type="button"
            variant="outline"
          >
            <ExternalLink aria-hidden="true" className="h-4 w-4" />
            {t("documentation.openRepository")}
          </Button>
        )}
        description={t("documentation.description")}
        icon={BookOpen}
        title={t("settings.pages.help")}
      />
      <SectionPanel description={t("documentation.readmeDescription")} title={t("documentation.readmeTitle")}>
        <RichMarkdown>{readme}</RichMarkdown>
      </SectionPanel>
    </div>
  );
}
