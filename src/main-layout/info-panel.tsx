import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";

export function InfoPanel() {
  return (
    <aside className="ucd-panel min-h-[620px] rounded-xl p-3">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">信息面板</h2>
        <Button className="h-7 px-2 text-xs" variant="outline">收起</Button>
      </div>
      <div className="mb-4 grid grid-cols-5 gap-1">
        {["Agent", "日志", "历史", "素材", "配置"].map((tab, index) => (
          <button
            className={cn(
              "h-8 rounded border border-border text-xs",
              index === 0 ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-muted-foreground",
            )}
            key={tab}
            type="button"
          >
            {tab}
          </button>
        ))}
      </div>

      <div className="grid gap-4">
        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">会话基础配置</h3>
          <div className="grid gap-3 text-sm">
            <label className="grid gap-1">
              <span className="text-muted-foreground">会话名称</span>
              <input className="ucd-input h-8 rounded px-2" defaultValue="智能客服优化方案" />
            </label>
            <label className="grid gap-1">
              <span className="text-muted-foreground">描述</span>
              <input className="ucd-input h-8 rounded px-2" placeholder="输入会话描述..." />
            </label>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">协作权限</span>
              <strong>可编辑</strong>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">自动保存</span>
              <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">已启用</span>
            </div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">全局模型参数</h3>
          <div className="grid gap-3 text-sm">
            <div className="flex items-center gap-3">
              <span className="w-16 text-muted-foreground">温度</span>
              <strong>0.7</strong>
              <div className="h-2 flex-1 rounded bg-muted"><div className="h-2 w-3/5 rounded bg-primary" /></div>
            </div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">最大上下文</span><strong>4096</strong></div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">输出格式</span><strong>文本</strong></div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">权限管理</h3>
          <div className="grid gap-2 text-sm">
            {[
              ["Z", "张三", "所有者"],
              ["L", "李四", "可编辑"],
              ["W", "王五", "只读"],
            ].map(([abbr, name, role]) => (
              <div className="flex items-center gap-2 rounded border border-border p-2" key={name}>
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-[hsl(var(--nav-active-soft))] text-xs font-semibold text-primary">{abbr}</span>
                <span>{name}</span>
                <span className="ml-auto text-xs text-muted-foreground">{role}</span>
              </div>
            ))}
            <button className="h-8 rounded border border-dashed border-border text-xs text-primary" type="button">+ 添加协作者</button>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">运行时信息</h3>
          <div className="grid grid-cols-2 gap-2 text-sm">
            {[
              ["Token 用量", "60%"],
              ["API 调用", "1,247"],
              ["预估费用", "$3.42"],
              ["运行时长", "02:34:18"],
            ].map(([label, value]) => (
              <div className="rounded border border-border p-2" key={label}>
                <div className="text-xs text-muted-foreground">{label}</div>
                <strong>{value}</strong>
              </div>
            ))}
          </div>
        </section>
      </div>
    </aside>
  );
}
