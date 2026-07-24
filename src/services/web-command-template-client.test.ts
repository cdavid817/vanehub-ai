import { describe, expect, it } from "vitest";
import {
  cancelWebCommandRun,
  deleteWebCommandTemplate,
  executeWebCommandTemplate,
  insertWebCommandTemplate,
  listWebCommandRuns,
  listWebCommandTemplates,
  saveWebCommandTemplate,
} from "./web-command-template-client";

describe("web command template adapter", () => {
  it("simulates insertion, execution, cancellation, pagination, and deletion snapshots", () => {
    saveWebCommandTemplate({ id: "template-test", name: "Build", command: "npm run build", scope: "global", connectionId: null, workspaceUri: null, workingDirectory: null });
    expect(insertWebCommandTemplate("template-test")).toBe("npm run build");
    const run = executeWebCommandTemplate("template-test", "session-test");
    expect(run.status).toBe("succeeded");
    expect(listWebCommandRuns("session-test", 0, 1)).toHaveLength(1);
    expect(cancelWebCommandRun(run.id)?.status).toBe("cancelled");
    deleteWebCommandTemplate("template-test");
    expect(listWebCommandTemplates()).not.toContainEqual(expect.objectContaining({ id: "template-test" }));
    expect(listWebCommandRuns("session-test")[0].commandSnapshot).toBe("npm run build");
  });
});
