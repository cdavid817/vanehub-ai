import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";

export function InfoPanel() {
  return (
    <aside className="ucd-panel min-h-[620px] rounded-xl p-3">
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">Info Panel</h2>
        <Button className="h-7 px-2 text-xs" variant="outline">Collapse</Button>
      </div>
      <div className="mb-4 grid grid-cols-5 gap-1">
        {["Agent", "Logs", "History", "Assets", "Config"].map((tab, index) => (
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
          <h3 className="mb-3 text-sm font-semibold">Session Configuration</h3>
          <div className="grid gap-3 text-sm">
            <label className="grid gap-1">
              <span className="text-muted-foreground">Session Name</span>
              <input className="ucd-input h-8 rounded px-2" defaultValue="Customer Support Optimization" />
            </label>
            <label className="grid gap-1">
              <span className="text-muted-foreground">Description</span>
              <input className="ucd-input h-8 rounded px-2" placeholder="Enter a session description..." />
            </label>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Collaboration Permission</span>
              <strong>Editable</strong>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Auto Save</span>
              <span className="rounded-full bg-[hsl(var(--success-soft))] px-2 py-1 text-xs text-[hsl(var(--success))]">Enabled</span>
            </div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">Global Model Parameters</h3>
          <div className="grid gap-3 text-sm">
            <div className="flex items-center gap-3">
              <span className="w-16 text-muted-foreground">Temperature</span>
              <strong>0.7</strong>
              <div className="h-2 flex-1 rounded bg-muted"><div className="h-2 w-3/5 rounded bg-primary" /></div>
            </div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">Max Context</span><strong>4096</strong></div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground">Output Format</span><strong>Text</strong></div>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">Permission Management</h3>
          <div className="grid gap-2 text-sm">
            {[
              ["O", "Owner", "Owner"],
              ["E", "Editor", "Editable"],
              ["R", "Reader", "Read-only"],
            ].map(([abbr, name, role]) => (
              <div className="flex items-center gap-2 rounded border border-border p-2" key={name}>
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-[hsl(var(--nav-active-soft))] text-xs font-semibold text-primary">{abbr}</span>
                <span>{name}</span>
                <span className="ml-auto text-xs text-muted-foreground">{role}</span>
              </div>
            ))}
            <button className="h-8 rounded border border-dashed border-border text-xs text-primary" type="button">+ Add Collaborator</button>
          </div>
        </section>

        <section className="ucd-muted-panel rounded-lg p-3">
          <h3 className="mb-3 text-sm font-semibold">Runtime Information</h3>
          <div className="grid grid-cols-2 gap-2 text-sm">
            {[
              ["Token Usage", "60%"],
              ["API Calls", "1,247"],
              ["Estimated Cost", "$3.42"],
              ["Runtime", "02:34:18"],
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
