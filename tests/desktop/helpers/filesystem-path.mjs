import { normalize } from "node:path";
import process from "node:process";

export function comparableFilesystemPath(value) {
  let path = value;
  if (process.platform === "win32") {
    path = path
      .replace(/^\\\\\?\\UNC\\/i, "\\\\")
      .replace(/^\\\\\?\\/i, "")
      .replaceAll("/", "\\");
    return normalize(path).toLowerCase();
  }
  return normalize(path);
}
