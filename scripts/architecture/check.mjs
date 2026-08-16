import path from "node:path";
import { fileURLToPath } from "node:url";
import { checkFrontendArchitecture } from "./frontend-rules.mjs";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const diagnostics = checkFrontendArchitecture(projectRoot);

if (diagnostics.length > 0) {
  process.stderr.write(`${diagnostics.join("\n")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("Architecture fitness: frontend and repository rules passed.\n");
}
