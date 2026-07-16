import { Plus } from "lucide-react";
import { Button } from "../../components/ui/button";
import { providers } from "../data/settings-demo-data";
import { PageHeader, SectionPanel, StatusPill, TagList } from "./page-parts";

export function ProvidersPage({ searchTerm }: { searchTerm: string }) {
  const visible = providers.filter((provider) => provider.name.toLowerCase().includes(searchTerm.toLowerCase()));

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button>
            <Plus className="h-4 w-4" aria-hidden="true" />
            Add Provider
          </Button>
        }
        description="Multi-model provider configuration, connection status, and routing policy"
        title="Provider Management"
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4 lg:grid-cols-2">
          {visible.map((provider) => (
            <section className="ucd-panel rounded-lg p-4" key={provider.id}>
              <div className="mb-3 flex items-start justify-between gap-3">
                <div>
                  <h3 className="font-semibold">{provider.name}</h3>
                  <p className="text-sm text-muted-foreground">{provider.vendor}</p>
                </div>
                <StatusPill status={provider.status} />
              </div>
              <TagList tags={provider.models} />
              <dl className="mt-4 grid gap-3 text-sm">
                <div>
                  <dt className="text-muted-foreground">API Key</dt>
                  <dd className="font-medium">{provider.key}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Endpoint</dt>
                  <dd className="break-all font-medium">{provider.endpoint}</dd>
                </div>
                <div className="flex justify-between gap-3">
                  <span>Average latency: {provider.latency}</span>
                  <span>Success rate: {provider.successRate}</span>
                </div>
              </dl>
            </section>
          ))}
        </div>
        <SectionPanel title="Routing Policy" description="Model routing and failover policy for multiple providers">
          <div className="grid gap-3 text-sm">
            {[
              ["Default provider", "Zhipu AI"],
              ["Load balancing", "Round robin"],
              ["Failover", "Enabled"],
              ["Timeout", "30 seconds"],
            ].map(([label, value]) => (
              <div className="flex justify-between gap-3 rounded border border-border p-3" key={label}>
                <span className="text-muted-foreground">{label}</span>
                <strong>{value}</strong>
              </div>
            ))}
          </div>
        </SectionPanel>
      </div>
    </div>
  );
}
