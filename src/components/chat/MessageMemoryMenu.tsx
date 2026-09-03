import { useMutation } from "@tanstack/react-query";
import { Brain } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService as defaultAgentService } from "../../services/runtime-agent-client";
import type { AgentService } from "../../services/agent-service";
import type { CreateMemoryInput } from "../../types/personalization-memory";
import { cn } from "../../lib/utils";
import { SurfacedMemoryPicker } from "./SurfacedMemoryPicker";

/** What the session knows that a memory written from one of its messages needs. */
export interface MessageMemoryContext {
  agentId: string;
  projectPath: string | null;
}

const NAME_WORDS = 6;

/**
 * A name derived from the text, because a menu action that opened a naming dialog would not be a
 * menu action any more. Non-ASCII slugs to nothing, so the fallback is what keeps a Chinese or
 * Japanese message from producing an empty name.
 */
export function deriveMemoryName(content: string, fallback: string): string {
  const slug = content
    .split(/[^a-zA-Z0-9]+/u)
    .filter(Boolean)
    .slice(0, NAME_WORDS)
    .join("-")
    .toLowerCase();
  return slug || fallback;
}

/**
 * Remember this message, or correct something already remembered.
 *
 * The two remember actions differ only in scope, and both write directly rather than proposing:
 * the user asking for it is the decision a proposal would have been waiting for.
 */
export function MessageMemoryMenu({
  content,
  context,
  service = defaultAgentService,
}: {
  content: string;
  context: MessageMemoryContext;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [picking, setPicking] = useState(false);
  const [saved, setSaved] = useState(false);
  const [failed, setFailed] = useState(false);

  const rememberMutation = useMutation({
    mutationFn: async (scope: "global" | "workspace") => {
      const input: CreateMemoryInput = {
        name: deriveMemoryName(content, `memory-${Date.now()}`),
        description: content.slice(0, 120),
        memoryType: scope === "workspace" ? "project" : "user",
        content,
        scopeKind: scope,
      };
      if (scope === "global") return service.createPersonalizationMemory(input);

      // Only the native side can turn a project path into a workspace key, and a memory stored
      // under a key this build invented would belong to a workspace nothing resolves to.
      const workspace = context.projectPath
        ? await service.resolvePersonalizationWorkspace({ projectPath: context.projectPath })
        : null;
      if (!workspace) throw new Error("personalization-workspace-unresolved");
      return service.createPersonalizationMemory({ ...input, workspaceKey: workspace.workspaceKey });
    },
    onSuccess: () => {
      setSaved(true);
      setFailed(false);
    },
    onError: () => {
      setSaved(false);
      setFailed(true);
    },
  });

  const busy = rememberMutation.isPending;

  return (
    <div className="mt-2 flex flex-col gap-2 border-t border-border/60 pt-2" data-testid="message-memory-menu">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <Brain aria-hidden="true" className="h-3.5 w-3.5 text-muted-foreground" />
        <MenuButton
          disabled={busy}
          onClick={() => rememberMutation.mutate("global")}
          testId="message-remember-global"
        >
          {t("chat.memory.rememberGlobally")}
        </MenuButton>
        <MenuButton
          // Without a project there is no workspace to scope to, and the action is disabled with a
          // reason rather than silently writing a global memory the user did not ask for.
          disabled={busy || !context.projectPath}
          onClick={() => rememberMutation.mutate("workspace")}
          testId="message-remember-project"
          title={context.projectPath ? undefined : t("chat.memory.noProject")}
        >
          {t("chat.memory.rememberForProject")}
        </MenuButton>
        <MenuButton disabled={busy} onClick={() => setPicking((open) => !open)} testId="message-forget-open">
          {t("chat.memory.forgetOrCorrect")}
        </MenuButton>
        {saved ? (
          <span aria-live="polite" className="text-muted-foreground" data-testid="message-memory-saved">
            {t("chat.memory.saved")}
          </span>
        ) : null}
        {failed ? (
          <span className="ucd-status-danger" data-testid="message-memory-failed" role="alert">
            {t("chat.memory.failed")}
          </span>
        ) : null}
      </div>

      {picking ? (
        <SurfacedMemoryPicker agentId={context.agentId} onClose={() => setPicking(false)} service={service} />
      ) : null}
    </div>
  );
}

function MenuButton({
  children,
  disabled,
  onClick,
  testId,
  title,
}: {
  children: React.ReactNode;
  disabled: boolean;
  onClick: () => void;
  testId: string;
  title?: string;
}) {
  return (
    <button
      className={cn(
        "rounded-md border border-border px-2 py-1 text-xs",
        disabled ? "opacity-50" : "hover:bg-muted",
      )}
      data-testid={testId}
      disabled={disabled}
      onClick={onClick}
      title={title}
      type="button"
    >
      {children}
    </button>
  );
}
