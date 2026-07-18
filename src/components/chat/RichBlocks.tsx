import {
  CheckCircle2,
  Circle,
  Code2,
  FileText,
  ImageIcon,
  Info,
  ListChecks,
  MousePointer2,
  Music2,
  PanelTop,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type {
  RichAudioBlock,
  RichBlock,
  RichCardBlock,
  RichChecklistBlock,
  RichDiffBlock,
  RichFileBlock,
  RichHtmlWidgetBlock,
  RichInteractiveBlock,
  RichMediaGalleryBlock,
} from "../../types/chat";

function BlockShell({
  children,
  icon,
  title,
  tone = "default",
}: {
  children: React.ReactNode;
  icon: React.ReactNode;
  title: string;
  tone?: "default" | "success" | "warning" | "danger";
}) {
  return (
    <section
      className={cn(
        "rounded-md border border-border bg-muted/50 text-xs text-foreground",
        tone === "success" && "border-[hsl(var(--success))]/40 bg-[hsl(var(--success-soft))]",
        tone === "warning" && "border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning-soft))]",
        tone === "danger" && "border-destructive/40 bg-destructive/10",
      )}
    >
      <div className="flex min-w-0 items-center gap-2 border-b border-border/70 px-3 py-2 text-muted-foreground">
        {icon}
        <span className="min-w-0 truncate font-medium">{title}</span>
      </div>
      <div className="px-3 py-2">{children}</div>
    </section>
  );
}

function CardBlock({ block }: { block: RichCardBlock }) {
  const tone = block.tone === "info" ? "default" : block.tone;
  return (
    <BlockShell icon={<Info className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title} tone={tone}>
      {block.bodyMarkdown ? (
        <div className="prose prose-sm max-w-none whitespace-normal text-foreground prose-p:my-1 prose-ul:my-1 prose-li:my-0">
          <ReactMarkdown>{block.bodyMarkdown}</ReactMarkdown>
        </div>
      ) : null}
      {block.fields?.length ? (
        <dl className="mt-2 grid gap-1">
          {block.fields.map((field) => (
            <div className="grid grid-cols-[minmax(5rem,0.35fr)_1fr] gap-2" key={field.label}>
              <dt className="truncate text-muted-foreground">{field.label}</dt>
              <dd className="min-w-0 break-words">{field.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
    </BlockShell>
  );
}

function DiffBlock({ block }: { block: RichDiffBlock }) {
  return (
    <BlockShell icon={<Code2 className="h-3.5 w-3.5" aria-hidden="true" />} title={block.filePath}>
      <pre className="max-h-80 overflow-auto rounded border border-border bg-background px-3 py-2 font-mono text-[11px] leading-5 text-muted-foreground">
        {block.diff}
      </pre>
    </BlockShell>
  );
}

function ChecklistBlock({ block }: { block: RichChecklistBlock }) {
  const { t } = useTranslation();
  const completed = block.items.filter((item) => item.checked).length;
  return (
    <BlockShell icon={<ListChecks className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title ?? t("chat.richBlock.checklist")}>
      <div className="mb-2 text-[11px] text-muted-foreground">
        {t("chat.richBlock.checklistProgress", { completed, total: block.items.length })}
      </div>
      <ul className="grid gap-1.5">
        {block.items.map((item) => (
          <li className="flex min-w-0 items-start gap-2" key={item.id}>
            {item.checked ? (
              <CheckCircle2 className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[hsl(var(--success))]" aria-hidden="true" />
            ) : (
              <Circle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
            )}
            <span className={cn("min-w-0 break-words", item.checked && "text-muted-foreground line-through")}>{item.text}</span>
          </li>
        ))}
      </ul>
    </BlockShell>
  );
}

function MediaGalleryBlock({ block }: { block: RichMediaGalleryBlock }) {
  const { t } = useTranslation();
  return (
    <BlockShell icon={<ImageIcon className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title ?? t("chat.richBlock.mediaGallery")}>
      <div className="grid gap-2 sm:grid-cols-2">
        {block.items.map((item) => (
          <figure className="min-w-0 overflow-hidden rounded border border-border bg-background" key={item.url}>
            <img className="max-h-64 w-full object-contain" src={item.url} alt={item.alt ?? item.caption ?? t("chat.richBlock.imageAlt")} />
            {item.caption ? <figcaption className="border-t border-border px-2 py-1 text-[11px] text-muted-foreground">{item.caption}</figcaption> : null}
          </figure>
        ))}
      </div>
    </BlockShell>
  );
}

function formatFileSize(value: number | undefined, label: string) {
  if (!value || value <= 0) return label;
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function FileBlock({ block }: { block: RichFileBlock }) {
  const { t } = useTranslation();
  return (
    <BlockShell icon={<FileText className="h-3.5 w-3.5" aria-hidden="true" />} title={block.fileName}>
      <a className="break-all text-primary underline-offset-4 hover:underline" href={block.url} target="_blank" rel="noreferrer">
        {block.url}
      </a>
      <div className="mt-1 text-[11px] text-muted-foreground">
        {block.mimeType ? `${block.mimeType} · ` : null}
        {formatFileSize(block.fileSize, t("chat.richBlock.fileSizeUnknown"))}
      </div>
    </BlockShell>
  );
}

function AudioBlock({ block }: { block: RichAudioBlock }) {
  const { t } = useTranslation();
  return (
    <BlockShell icon={<Music2 className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title ?? t("chat.richBlock.audio")}>
      {block.text ? <p className="mb-2 whitespace-pre-wrap leading-5">{block.text}</p> : null}
      <audio className="w-full" controls src={block.url}>
        {t("chat.richBlock.audioUnsupported")}
      </audio>
    </BlockShell>
  );
}

function HtmlWidgetBlock({ block }: { block: RichHtmlWidgetBlock }) {
  const { t } = useTranslation();
  const height = Math.min(Math.max(block.height ?? 300, 50), 600);
  return (
    <BlockShell icon={<PanelTop className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title ?? t("chat.richBlock.htmlWidget")}>
      <iframe
        className="w-full rounded border border-border bg-background"
        height={height}
        sandbox=""
        srcDoc={block.html}
        title={block.title ?? t("chat.richBlock.htmlWidget")}
      />
    </BlockShell>
  );
}

function InteractiveBlock({ block }: { block: RichInteractiveBlock }) {
  const { t } = useTranslation();
  return (
    <BlockShell icon={<MousePointer2 className="h-3.5 w-3.5" aria-hidden="true" />} title={block.title ?? t("chat.richBlock.interactive")}>
      {block.description ? <p className="mb-2 text-muted-foreground">{block.description}</p> : null}
      <div className="grid gap-1.5">
        {block.options.map((option) => (
          <div className="rounded border border-border bg-background px-2 py-1.5" key={option.id}>
            <div className="font-medium">{option.label}</div>
            {option.description ? <div className="mt-0.5 text-[11px] text-muted-foreground">{option.description}</div> : null}
          </div>
        ))}
      </div>
      <p className="mt-2 text-[11px] text-muted-foreground">{t("chat.richBlock.interactiveDisabled")}</p>
    </BlockShell>
  );
}

function UnknownBlock({ block }: { block: { kind?: unknown } }) {
  const { t } = useTranslation();
  const kind = typeof block.kind === "string" ? block.kind : t("chat.richBlock.unknownKind");
  return (
    <BlockShell icon={<Info className="h-3.5 w-3.5" aria-hidden="true" />} title={t("chat.richBlock.unsupportedTitle")}>
      <p className="text-muted-foreground">{t("chat.richBlock.unsupportedBody", { kind })}</p>
    </BlockShell>
  );
}

function RichBlockRenderer({ block }: { block: RichBlock }) {
  switch (block.kind) {
    case "card":
      return <CardBlock block={block} />;
    case "diff":
      return <DiffBlock block={block} />;
    case "checklist":
      return <ChecklistBlock block={block} />;
    case "media_gallery":
      return <MediaGalleryBlock block={block} />;
    case "file":
      return <FileBlock block={block} />;
    case "audio":
      return <AudioBlock block={block} />;
    case "html_widget":
      return <HtmlWidgetBlock block={block} />;
    case "interactive":
      return <InteractiveBlock block={block} />;
    default:
      return <UnknownBlock block={block as { kind?: unknown }} />;
  }
}

export function RichBlocks({ blocks }: { blocks: RichBlock[] }) {
  if (blocks.length === 0) return null;
  return (
    <div className="mt-3 grid gap-2">
      {blocks.map((block) => (
        <RichBlockRenderer block={block} key={block.id} />
      ))}
    </div>
  );
}
