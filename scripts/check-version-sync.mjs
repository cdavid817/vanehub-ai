import { readFile } from "node:fs/promises";
import console from "node:console";
import process from "node:process";

const packageJson = JSON.parse(await readFile("package.json", "utf8"));
const tauriConfig = JSON.parse(
  await readFile("src-tauri/tauri.conf.json", "utf8"),
);
const cargoToml = await readFile("src-tauri/Cargo.toml", "utf8");
const cargoPackage = cargoToml.match(
  /\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
);

if (!cargoPackage) {
  throw new Error("Could not read the [package] version from src-tauri/Cargo.toml");
}

const versions = new Map([
  ["package.json", packageJson.version],
  ["src-tauri/Cargo.toml", cargoPackage[1]],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
]);
const uniqueVersions = new Set(versions.values());

if (uniqueVersions.size !== 1) {
  const details = [...versions]
    .map(([file, version]) => `${file}: ${String(version)}`)
    .join("\n");
  throw new Error(`Project versions are not synchronized:\n${details}`);
}

const version = packageJson.version;
const suppliedTag = process.argv[2] ?? process.env.GITHUB_REF_NAME;
if (suppliedTag && suppliedTag !== `v${version}`) {
  throw new Error(
    `Release tag ${suppliedTag} does not match synchronized version v${version}`,
  );
}

console.log(`Version ${version} is synchronized across npm, Cargo, and Tauri.`);
