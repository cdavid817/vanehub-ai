import { Circle, Minus, Plus } from "lucide-react";

export function StatusBar() {
  return (
    <footer className="mx-2 mb-2 flex min-h-8 flex-wrap items-center justify-between gap-2 rounded border border-border px-3 text-xs text-muted-foreground">
      <div className="flex items-center gap-3">
        <span className="inline-flex items-center gap-1"><Circle className="h-3 w-3 fill-[hsl(var(--success))] text-[hsl(var(--success))]" />已连接</span>
        <span>状态: 空闲</span>
        <span>Token: 2,340</span>
        <span>调用: 15</span>
      </div>
      <div className="flex items-center gap-3">
        <button className="inline-flex items-center gap-1" type="button"><Plus className="h-3 w-3" />100%</button>
        <button type="button"><Minus className="h-3 w-3" /></button>
        <span>已自动保存</span>
        <span>v0.1.0</span>
      </div>
    </footer>
  );
}
