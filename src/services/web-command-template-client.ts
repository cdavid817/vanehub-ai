export type WebCommandTemplateScope = "global" | "connection" | "workspace";
export type WebCommandRunStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface WebCommandTemplate { id: string; name: string; command: string; scope: WebCommandTemplateScope; connectionId: string | null; workspaceUri: string | null; workingDirectory: string | null; }
export interface WebCommandRun { id: string; templateId: string | null; sessionId: string; connectionId: string | null; commandSnapshot: string; workingDirectory: string | null; status: WebCommandRunStatus; exitCode: number | null; startedAt: string; finishedAt: string | null; }

const templates = new Map<string, WebCommandTemplate>();
const runs = new Map<string, WebCommandRun>();

export function listWebCommandTemplates(scope?: WebCommandTemplateScope): WebCommandTemplate[] { return [...templates.values()].filter((template) => !scope || template.scope === scope); }
export function saveWebCommandTemplate(template: WebCommandTemplate): WebCommandTemplate { templates.set(template.id, { ...template }); return { ...template }; }
export function deleteWebCommandTemplate(id: string): void { templates.delete(id); for (const run of runs.values()) if (run.templateId === id) run.templateId = null; }
export function insertWebCommandTemplate(id: string): string { const template = templates.get(id); if (!template) throw new Error("Command template not found"); return template.command; }
export function executeWebCommandTemplate(templateId: string, sessionId: string): WebCommandRun { const template = templates.get(templateId); if (!template) throw new Error("Command template not found"); const now = new Date().toISOString(); const run: WebCommandRun = { id: `web-run-${runs.size + 1}`, templateId, sessionId, connectionId: template.connectionId, commandSnapshot: template.command, workingDirectory: template.workingDirectory, status: "succeeded", exitCode: 0, startedAt: now, finishedAt: now }; runs.set(run.id, run); return { ...run }; }
export function cancelWebCommandRun(id: string): WebCommandRun | null { const run = runs.get(id); if (!run) return null; run.status = "cancelled"; run.exitCode = null; run.finishedAt = new Date().toISOString(); return { ...run }; }
export function listWebCommandRuns(sessionId: string, offset = 0, limit = 50): WebCommandRun[] { return [...runs.values()].filter((run) => run.sessionId === sessionId).slice(offset, offset + Math.min(Math.max(limit, 1), 100)).map((run) => ({ ...run })); }
