import { Plus, Upload } from "lucide-react";
import { Button } from "../../components/ui/button";
import { skills } from "../data/settings-demo-data";
import { PageHeader, SectionPanel, StatCard, TagList } from "./page-parts";

export function SkillsPage({ searchTerm }: { searchTerm: string }) {
  const visible = skills.filter((skill) => skill.name.toLowerCase().includes(searchTerm.toLowerCase()));
  const active = visible[0] ?? skills[0];

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button variant="outline">
              <Upload className="h-4 w-4" aria-hidden="true" />
              导入 Skill
            </Button>
            <Button>
              <Plus className="h-4 w-4" aria-hidden="true" />
              创建 Skill
            </Button>
          </>
        }
        description="管理可复用能力、触发命令和 Agent 关联"
        title="Skills"
      />
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="全部" value="8" hint="已注册 Skills" />
        <StatCard label="已启用" value="6" hint="可被 Agent 调用" />
        <StatCard label="已禁用" value="2" hint="暂不参与路由" />
        <StatCard label="平均成功率" value="91%" hint="最近运行统计" />
      </div>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {visible.map((skill) => (
            <section className="ucd-panel rounded-lg p-4" key={skill.id}>
              <div className="mb-3 flex items-start justify-between gap-3">
                <div>
                  <h3 className="font-semibold">{skill.name}</h3>
                  <p className="text-sm text-muted-foreground">触发: {skill.trigger}</p>
                </div>
                <span className="rounded-sm bg-[hsl(var(--nav-active-soft))] px-2 py-1 text-xs font-medium text-primary">
                  {skill.category}
                </span>
              </div>
              <div className="text-sm text-muted-foreground">Agent: {skill.agent}</div>
              <div className="mt-4 flex justify-between gap-3 text-sm">
                <span>{skill.runs}次</span>
                <strong>{skill.success}</strong>
              </div>
            </section>
          ))}
        </div>
        <SectionPanel title="Skill 配置详情" description="当前选中 Skill 的元数据">
          <dl className="grid gap-3 text-sm">
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">Skill ID</dt><dd className="font-medium">{active.id}</dd></div>
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">版本</dt><dd className="font-medium">1.2.0</dd></div>
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">作者</dt><dd className="font-medium">VaneHub Team</dd></div>
            <div>
              <dt className="mb-2 text-muted-foreground">分类标签</dt>
              <TagList tags={[active.category, "自动化", "工作流"]} />
            </div>
          </dl>
        </SectionPanel>
      </div>
    </div>
  );
}
