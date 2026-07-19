import { Braces, Code2, FolderOpen, PanelsTopLeft, SquareTerminal, TerminalSquare, type LucideIcon } from "lucide-react";
import type { FolderOpenerId } from "../types/folder-opener";

const icons: Record<FolderOpenerId, LucideIcon> = {
  vscode: Code2,
  "file-explorer": FolderOpen,
  "windows-terminal": SquareTerminal,
  "git-bash": TerminalSquare,
  "intellij-idea": Braces,
  webstorm: PanelsTopLeft,
};

export function FolderOpenerIcon({ id, className = "h-4 w-4" }: { id: FolderOpenerId; className?: string }) {
  const Icon = icons[id];
  return <Icon aria-hidden="true" className={className} />;
}
