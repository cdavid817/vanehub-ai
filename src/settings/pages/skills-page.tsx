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
              Import Skill
            </Button>
            <Button>
              <Plus className="h-4 w-4" aria-hidden="true" />
              Create Skill
            </Button>
          </>
        }
        description="Manage reusable capabilities, trigger commands, and Agent bindings"
        title="Skills"
      />
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="All" value="8" hint="Registered Skills" />
        <StatCard label="Enabled" value="6" hint="Available to Agents" />
        <StatCard label="Disabled" value="2" hint="Excluded from routing" />
        <StatCard label="Average Success" value="91%" hint="Recent run statistics" />
      </div>
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {visible.map((skill) => (
            <section className="ucd-panel rounded-lg p-4" key={skill.id}>
              <div className="mb-3 flex items-start justify-between gap-3">
                <div>
                  <h3 className="font-semibold">{skill.name}</h3>
                  <p className="text-sm text-muted-foreground">Trigger: {skill.trigger}</p>
                </div>
                <span className="rounded-sm bg-[hsl(var(--nav-active-soft))] px-2 py-1 text-xs font-medium text-primary">
                  {skill.category}
                </span>
              </div>
              <div className="text-sm text-muted-foreground">Agent: {skill.agent}</div>
              <div className="mt-4 flex justify-between gap-3 text-sm">
                <span>{skill.runs} runs</span>
                <strong>{skill.success}</strong>
              </div>
            </section>
          ))}
        </div>
        <SectionPanel title="Skill Details" description="Metadata for the selected Skill">
          <dl className="grid gap-3 text-sm">
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">Skill ID</dt><dd className="font-medium">{active.id}</dd></div>
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">Version</dt><dd className="font-medium">1.2.0</dd></div>
            <div className="flex justify-between gap-3"><dt className="text-muted-foreground">Author</dt><dd className="font-medium">VaneHub Team</dd></div>
            <div>
              <dt className="mb-2 text-muted-foreground">Tags</dt>
              <TagList tags={[active.category, "Automation", "Workflow"]} />
            </div>
          </dl>
        </SectionPanel>
      </div>
    </div>
  );
}
