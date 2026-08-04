import { Check, ExternalLink } from "lucide-react";
import type { ProviderEndpointType } from "../../types/agent";
import { Badge } from "../ui/badge";
import type { ReactNode } from "react";

const endpointLabels: Record<ProviderEndpointType, string> = {
  "anthropic-messages": "Anthropic Messages",
  "openai-chat-completions": "OpenAI Chat Completions",
  "openai-responses": "OpenAI Responses",
};

export function ProviderEndpointBadge({ type }: { type: ProviderEndpointType }) {
  return <Badge tone={type === "anthropic-messages" ? "success" : "muted"}>{endpointLabels[type]}</Badge>;
}

export function ProviderEndpointSelector({ endpoints, value, onChange, label, disabled = false }: {
  endpoints: Array<{ type: ProviderEndpointType; baseUrl: string }>;
  value: ProviderEndpointType;
  onChange: (type: ProviderEndpointType) => void;
  label: string;
  disabled?: boolean;
}) {
  return <fieldset className="rounded-xl border border-border bg-background p-3 sm:p-4" disabled={disabled}>
    <legend className="px-1 text-sm font-semibold">{label}</legend>
    <div className="grid gap-2 sm:grid-cols-2">
      {endpoints.map((endpoint) => {
        const selected = endpoint.type === value;
        return <button aria-pressed={selected} className={`relative min-w-0 rounded-lg border p-3 text-left ${selected ? "border-primary bg-primary/5 ring-1 ring-primary/30" : "border-border hover:bg-muted/40"} disabled:cursor-not-allowed disabled:opacity-70`} disabled={disabled} key={endpoint.type} onClick={() => onChange(endpoint.type)} type="button">
          {selected ? <Check className="absolute right-2 top-2 h-4 w-4 text-primary" /> : null}
          <ProviderEndpointBadge type={endpoint.type} />
          <span className="mt-2 block truncate font-mono text-xs text-muted-foreground" title={endpoint.baseUrl}>{endpoint.baseUrl}</span>
        </button>;
      })}
    </div>
  </fieldset>;
}

export function ProviderHelpLinks({ apiKeyUrl, docsUrl, apiKeyLabel, docsLabel, onOpenUrl }: { apiKeyUrl: string; docsUrl: string; apiKeyLabel: string; docsLabel: string; onOpenUrl: (url: string) => void }) {
  return <div className="flex flex-wrap gap-x-4 gap-y-2 text-sm">
    <a className="inline-flex items-center gap-1 text-primary hover:underline" href={apiKeyUrl} onClick={(event) => { event.preventDefault(); onOpenUrl(apiKeyUrl); }}>{apiKeyLabel}<ExternalLink className="h-3.5 w-3.5" /></a>
    <a className="inline-flex items-center gap-1 text-primary hover:underline" href={docsUrl} onClick={(event) => { event.preventDefault(); onOpenUrl(docsUrl); }}>{docsLabel}<ExternalLink className="h-3.5 w-3.5" /></a>
  </div>;
}

export function ProviderConfigurationSection({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return <section className="rounded-xl border border-border bg-muted/10 p-3 sm:p-4">
    <h3 className="text-sm font-semibold">{title}</h3>
    <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
    <div className="mt-4 grid gap-4">{children}</div>
  </section>;
}
