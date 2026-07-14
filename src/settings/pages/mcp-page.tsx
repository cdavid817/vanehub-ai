import { PlugZap } from "lucide-react";
import { Button } from "../../components/ui/button";
import { mcpServers } from "../data/settings-demo-data";
import { PageHeader, SectionPanel, StatCard, StatusPill } from "./page-parts";

export function McpPage({ searchTerm }: { searchTerm: string }) {
  const visible = mcpServers.filter((server) => server.name.toLowerCase().includes(searchTerm.toLowerCase()));

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button>
            <PlugZap className="h-4 w-4" aria-hidden="true" />
            添加 MCP
          </Button>
        }
        description="管理 MCP 服务器连接、工具数量和运行状态"
        title="MCP 服务器"
      />
      <div className="grid gap-4 md:grid-cols-3">
        <StatCard label="运行中" value="2" hint="服务响应正常" />
        <StatCard label="工具总数" value="26" hint="跨服务器可用工具" />
        <StatCard label="平均延迟" value="49ms" hint="最近 15 分钟" />
      </div>
      <div className="grid gap-4 lg:grid-cols-3">
        {visible.map((server) => (
          <section className="ucd-panel rounded-lg p-4" key={server.id}>
            <div className="mb-4 flex items-start justify-between gap-3">
              <div>
                <h3 className="font-semibold">{server.name}</h3>
                <p className="text-sm text-muted-foreground">{server.scope}</p>
              </div>
              <StatusPill status={server.status} />
            </div>
            <dl className="grid gap-3 text-sm">
              <div className="flex justify-between gap-3">
                <dt className="text-muted-foreground">工具数量</dt>
                <dd className="font-medium">{server.tools}</dd>
              </div>
              <div className="flex justify-between gap-3">
                <dt className="text-muted-foreground">延迟</dt>
                <dd className="font-medium">{server.latency}</dd>
              </div>
            </dl>
          </section>
        ))}
      </div>
      <SectionPanel title="连接策略" description="服务器健康检查和故障切换配置">
        <div className="grid gap-3 text-sm md:grid-cols-3">
          <div className="rounded border border-border p-3">健康检查 <strong className="block">30 秒</strong></div>
          <div className="rounded border border-border p-3">失败重试 <strong className="block">2 次</strong></div>
          <div className="rounded border border-border p-3">自动恢复 <strong className="block">已启用</strong></div>
        </div>
      </SectionPanel>
    </div>
  );
}
