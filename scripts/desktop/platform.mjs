import { DesktopVerificationError } from "./verification-error.mjs";

const HOSTS = new Map([
  ["win32:x64", { platform: "windows", architecture: "x64", targetTriple: "x86_64-pc-windows-msvc", extension: ".exe" }],
  ["win32:arm64", { platform: "windows", architecture: "arm64", targetTriple: "aarch64-pc-windows-msvc", extension: ".exe" }],
  ["darwin:x64", { platform: "macos", architecture: "x64", targetTriple: "x86_64-apple-darwin", extension: "" }],
  ["darwin:arm64", { platform: "macos", architecture: "arm64", targetTriple: "aarch64-apple-darwin", extension: "" }],
  ["linux:x64", { platform: "linux", architecture: "x64", targetTriple: "x86_64-unknown-linux-gnu", extension: "" }],
  ["linux:arm64", { platform: "linux", architecture: "arm64", targetTriple: "aarch64-unknown-linux-gnu", extension: "" }],
]);

export function detectHost(platform = process.platform, architecture = process.arch) {
  const host = HOSTS.get(`${platform}:${architecture}`);
  if (!host) {
    throw new DesktopVerificationError(
      "BLOCKED",
      `Desktop verification does not support ${platform}/${architecture}.`,
      { platform, architecture },
    );
  }
  return { ...host };
}
