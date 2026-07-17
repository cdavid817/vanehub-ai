import type { GitDiffFile } from "../types/session-workspace";
import { cn } from "../lib/utils";

export type DiffViewMode = "unified" | "split";

export function buildUnifiedRows(file: GitDiffFile) {
  return file.hunks.flatMap((hunk) => hunk.lines.map((line, index) => ({ ...line, key: `${hunk.header}-${index}` })));
}

export function buildSplitRows(file: GitDiffFile) {
  return buildUnifiedRows(file).map((line) => ({
    key: line.key,
    left: line.kind === "addition" ? { content: "", kind: "context", number: null } : { content: line.content, kind: line.kind, number: line.oldLineNumber },
    right: line.kind === "deletion" ? { content: "", kind: "context", number: null } : { content: line.content, kind: line.kind, number: line.newLineNumber },
  }));
}

export function DiffView({ file, mode }: { file: GitDiffFile; mode: DiffViewMode }) {
  if (mode === "split") {
    return <div className="grid gap-3">{file.hunks.map((hunk) => <section className="overflow-auto rounded border border-border" key={hunk.header}><HunkHeader header={hunk.header} /><div className="grid min-w-[640px] grid-cols-2">{buildSplitRows({ ...file, hunks: [hunk] }).map((row) => <div className="col-span-2 grid grid-cols-2" key={row.key}><DiffCell {...row.left} /><DiffCell {...row.right} /></div>)}</div></section>)}</div>;
  }
  return <div className="grid gap-3">{file.hunks.map((hunk) => <section className="overflow-auto rounded border border-border" key={hunk.header}><HunkHeader header={hunk.header} />{hunk.lines.map((line, index) => <div className={cn("grid min-w-[520px] grid-cols-[48px_48px_minmax(0,1fr)] font-mono text-xs", line.kind === "addition" && "bg-[hsl(var(--success-soft))]", line.kind === "deletion" && "bg-destructive/10")} key={`${hunk.header}-${index}`}><span className="border-r border-border px-2 py-1 text-right text-muted-foreground">{line.oldLineNumber}</span><span className="border-r border-border px-2 py-1 text-right text-muted-foreground">{line.newLineNumber}</span><span className="whitespace-pre px-2 py-1">{line.kind === "addition" ? "+" : line.kind === "deletion" ? "-" : " "}{line.content}</span></div>)}</section>)}</div>;
}

function HunkHeader({ header }: { header: string }) { return <div className="bg-muted px-2 py-1 font-mono text-xs text-primary">{header}</div>; }
function DiffCell({ content, kind, number }: { content: string; kind: string; number: number | null }) { return <div className={cn("grid grid-cols-[48px_minmax(0,1fr)] border-r border-border font-mono text-xs", kind === "addition" && "bg-[hsl(var(--success-soft))]", kind === "deletion" && "bg-destructive/10")}><span className="border-r border-border px-2 py-1 text-right text-muted-foreground">{number}</span><span className="whitespace-pre px-2 py-1">{content}</span></div>; }
