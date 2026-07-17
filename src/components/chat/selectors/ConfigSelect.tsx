import { Bot, Settings, ToggleLeft, ToggleRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../../../types/agent";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

export function ConfigSelect({
  agents,
  longContext,
  onAgentChange,
  onClose,
  onLongContextChange,
  onOpen,
  onStreamingChange,
  onThinkingChange,
  open,
  selectedAgentId,
  streaming,
  thinking,
}: {
  agents: AgentRegistryEntry[];
  longContext: boolean;
  onAgentChange: (value: string) => void;
  onClose: () => void;
  onLongContextChange: (value: boolean) => void;
  onOpen: () => void;
  onStreamingChange: (value: boolean) => void;
  onThinkingChange: (value: boolean) => void;
  open: boolean;
  selectedAgentId: string;
  streaming: boolean;
  thinking: boolean;
}) {
  const { t } = useTranslation();
  return (
    <div className="relative">
      <SelectorButton compact icon={<Settings className="h-3.5 w-3.5" aria-hidden="true" />} label={t("chat.config.configure")} onClick={onOpen} open={open} title={t("chat.config.configure")} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onAgentChange}
          options={agents.map((agent) => ({
            value: agent.id,
            label: agent.displayName,
            description: agent.provider,
            icon: <Bot className="h-3.5 w-3.5" aria-hidden="true" />,
          }))}
          value={selectedAgentId}
        >
          <div className="my-1 h-px bg-border" />
          {[
            [t("chat.config.streaming"), streaming, onStreamingChange],
            [t("chat.config.thinking"), thinking, onThinkingChange],
            [t("chat.config.longContext"), longContext, onLongContextChange],
          ].map(([label, checked, onChange]) => (
            <button
              className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-xs hover:bg-muted"
              key={label as string}
              onClick={() => (onChange as (value: boolean) => void)(!(checked as boolean))}
              type="button"
            >
              {checked ? <ToggleRight className="h-4 w-4 text-primary" /> : <ToggleLeft className="h-4 w-4 text-muted-foreground" />}
              <span>{label as string}</span>
              <span className="ml-auto text-muted-foreground">{checked ? t("chat.config.on") : t("chat.config.off")}</span>
            </button>
          ))}
        </SelectorDropdown>
      ) : null}
    </div>
  );
}
