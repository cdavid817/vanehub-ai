import { RotateCcw, Save } from "lucide-react";
import { Button } from "../../components/ui/button";
import { PageHeader, SectionPanel } from "./page-parts";

export function BasicSettingsPage() {
  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button variant="outline">
              <RotateCcw className="h-4 w-4" aria-hidden="true" />
              重置默认
            </Button>
            <Button>
              <Save className="h-4 w-4" aria-hidden="true" />
              保存
            </Button>
          </>
        }
        description="应用程序的基础参数与行为配置"
        title="通用设置"
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="space-y-4">
          <SectionPanel title="应用设置" description="基础偏好与启动行为">
            <div className="grid gap-4 md:grid-cols-2">
              {[
                ["应用名称", "VaneHub AI"],
                ["应用语言", "简体中文"],
                ["字体大小", "14px"],
                ["日志级别", "INFO"],
              ].map(([label, value]) => (
                <label className="grid gap-1 text-sm" key={label}>
                  <span className="text-muted-foreground">{label}</span>
                  <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" defaultValue={value} />
                </label>
              ))}
            </div>
            <div className="mt-4 grid gap-3">
              {["自动保存 - 每 30 秒自动保存会话", "启动恢复 - 启动时恢复上次会话状态", "自动检查更新 - 每日检查新版本", "匿名使用统计 - 帮助改进产品"].map((item) => (
                <label className="flex items-center justify-between gap-3 rounded-md border border-border p-3 text-sm" key={item}>
                  <span>{item}</span>
                  <input defaultChecked className="h-4 w-4 accent-[hsl(var(--primary))]" type="checkbox" />
                </label>
              ))}
            </div>
          </SectionPanel>

          <SectionPanel title="模型参数" description="全局默认模型推理参数，可在会话级别覆盖">
            <div className="grid gap-4 md:grid-cols-3">
              {[
                ["默认模型", "GLM-4.5"],
                ["备用模型", "Claude-3.5"],
                ["温度", "0.7"],
                ["最大输出 Token", "4096"],
                ["上下文窗口", "8192"],
                ["Top P", "0.9"],
              ].map(([label, value]) => (
                <label className="grid gap-1 text-sm" key={label}>
                  <span className="text-muted-foreground">{label}</span>
                  <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" defaultValue={value} />
                </label>
              ))}
            </div>
          </SectionPanel>
        </div>

        <SectionPanel title="数据与存储" description="数据存储路径与缓存管理">
          <div className="grid gap-4 text-sm">
            <div>
              <div className="text-muted-foreground">数据目录</div>
              <div className="mt-1 break-all font-medium">C:\Users\c00606997\.vanehub-ai\data</div>
            </div>
            <div className="ucd-muted-panel rounded p-3">
              <div className="flex justify-between gap-3">
                <span>缓存大小</span>
                <strong>256 MB</strong>
              </div>
              <div className="mt-3 h-2 rounded bg-muted">
                <div className="h-2 w-1/4 rounded bg-primary" />
              </div>
              <div className="mt-2 text-xs text-muted-foreground">256 MB / 1 GB</div>
            </div>
            <Button variant="outline">清理缓存</Button>
          </div>
        </SectionPanel>
      </div>
    </div>
  );
}
