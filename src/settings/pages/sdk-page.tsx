import { Download, RefreshCw } from "lucide-react";
import { Button } from "../../components/ui/button";
import { sdkPackages } from "../data/settings-demo-data";
import { PageHeader, SectionPanel, StatCard, StatusPill } from "./page-parts";

export function SdkPage({ searchTerm }: { searchTerm: string }) {
  const visible = sdkPackages.filter((pkg) => pkg.name.toLowerCase().includes(searchTerm.toLowerCase()));

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button variant="outline">
              <RefreshCw className="h-4 w-4" aria-hidden="true" />
              检查更新
            </Button>
            <Button>
              <Download className="h-4 w-4" aria-hidden="true" />
              安装 SDK
            </Button>
          </>
        }
        description="SDK 安装、版本状态与依赖关系概览"
        title="SDK 依赖"
      />
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="SDK 已安装" value="5" hint="总占用 128 MB" />
        <StatCard label="SDK 可更新" value="2" hint="建议及时更新" />
        <StatCard label="SDK 为最新" value="3" hint="运行状态良好" />
        <StatCard label="SDK 异常" value="0" hint="无缺失依赖" />
      </div>
      <SectionPanel title="SDK 列表">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead className="border-b border-border text-xs uppercase text-muted-foreground">
              <tr>
                {["SDK 名称", "当前版本", "最新版本", "状态", "大小", "来源", "操作"].map((head) => (
                  <th className="px-3 py-2 font-medium" key={head}>{head}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {visible.map((pkg) => (
                <tr className="border-b border-border last:border-0" key={pkg.name}>
                  <td className="px-3 py-3">
                    <div className="font-medium">{pkg.name}</div>
                    <div className="text-xs text-muted-foreground">{pkg.description}</div>
                  </td>
                  <td className="px-3 py-3">{pkg.current}</td>
                  <td className="px-3 py-3">{pkg.latest}</td>
                  <td className="px-3 py-3"><StatusPill status={pkg.status} /></td>
                  <td className="px-3 py-3">{pkg.size}</td>
                  <td className="px-3 py-3">{pkg.source}</td>
                  <td className="px-3 py-3"><Button size="default" variant="outline">详情</Button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </SectionPanel>
      <SectionPanel title="安装配置" description="SDK 安装源与镜像配置">
        <div className="grid gap-3 text-sm md:grid-cols-3">
          <div className="rounded border border-border p-3">安装源 <strong className="block">华为内网镜像</strong></div>
          <div className="rounded border border-border p-3 md:col-span-2">镜像地址 <strong className="block break-all">https://repo.huawei.com/npm/repository/npm-group/</strong></div>
        </div>
      </SectionPanel>
    </div>
  );
}
