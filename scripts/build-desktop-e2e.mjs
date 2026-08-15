import { spawnSync } from "node:child_process";

const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error("npm_execpath is required to build the desktop E2E frontend.");
const result = spawnSync(process.execPath, [npmCli, "run", "build"], {
  env: { ...process.env, VITE_DESKTOP_E2E: "1" },
  stdio: "inherit",
});

if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
