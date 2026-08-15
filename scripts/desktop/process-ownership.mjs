import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { DesktopVerificationError } from "./verification-error.mjs";

export async function readProcessMarker(dataDir) {
  const markerPath = path.join(dataDir, "desktop-e2e-process.json");
  const marker = JSON.parse(await readFile(markerPath, "utf8"));
  if (!Number.isInteger(marker.pid) || marker.pid <= 0 || typeof marker.runId !== "string") {
    throw new DesktopVerificationError("FAILED", "The desktop process marker is invalid.", { markerPath });
  }
  return { ...marker, markerPath };
}

export function isProcessRunning(pid, signal = process.kill) {
  try {
    signal(pid, 0);
    return true;
  } catch (error) {
    return error?.code === "EPERM";
  }
}

export function ownedProcessIds(rootPid, processes) {
  const owned = new Set([rootPid]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const processInfo of processes) {
      if (owned.has(processInfo.parentPid) && !owned.has(processInfo.pid)) {
        owned.add(processInfo.pid);
        changed = true;
      }
    }
  }
  return [...owned];
}

export function readProcessTable(execute = execFileSync) {
  if (process.platform === "win32") {
    const script = "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId | ConvertTo-Json -Compress";
    const raw = execute("powershell", ["-NoProfile", "-Command", script], { encoding: "utf8" });
    const rows = JSON.parse(raw || "[]");
    return (Array.isArray(rows) ? rows : [rows]).map((row) => ({
      pid: Number(row.ProcessId),
      parentPid: Number(row.ParentProcessId),
    }));
  }
  const raw = execute("ps", ["-eo", "pid=,ppid="], { encoding: "utf8" });
  return raw.trim().split("\n").filter(Boolean).map((line) => {
    const [pid, parentPid] = line.trim().split(/\s+/).map(Number);
    return { pid, parentPid };
  });
}

export async function ensureOwnedProcessesStopped({ marker, runId, timeoutMs = 12_000, pollMs = 200 }) {
  if (marker.runId !== runId) {
    throw new DesktopVerificationError("FAILED", "Process ownership could not be proven for cleanup.", {
      expectedRunId: runId,
      markerRunId: marker.runId,
    });
  }
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline && isProcessRunning(marker.pid)) {
    await delay(pollMs);
  }
  if (!isProcessRunning(marker.pid)) return { forced: false, remaining: [] };

  const owned = ownedProcessIds(marker.pid, readProcessTable()).reverse();
  for (const pid of owned) {
    if (isProcessRunning(pid)) process.kill(pid, "SIGTERM");
  }
  const cleanupDeadline = Date.now() + timeoutMs;
  let remaining = owned.filter((pid) => isProcessRunning(pid));
  while (remaining.length > 0 && Date.now() < cleanupDeadline) {
    await delay(pollMs);
    remaining = owned.filter((pid) => isProcessRunning(pid));
  }
  if (remaining.length > 0) {
    throw new DesktopVerificationError("FAILED", "Owned desktop processes remained after bounded cleanup.", { remaining });
  }
  return { forced: true, remaining: [] };
}
