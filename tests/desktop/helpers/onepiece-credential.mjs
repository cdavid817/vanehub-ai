import { Buffer } from "node:buffer";
import { execFileSync } from "node:child_process";
import process from "node:process";

/**
 * Resolves an OnePiece provider API key for the desktop verification run, or `null` when the
 * host has not supplied one.
 *
 * The harness runs against an isolated, empty application data directory, so the installed app's
 * provider profile — and the OS keyring entry keyed by that profile's id — is not reachable from
 * the run. A host that wants the live-provider case covered supplies the key one of two ways:
 *
 * - `VANEHUB_ONEPIECE_API_KEY`, which is what CI would use.
 * - `VANEHUB_ONEPIECE_PROFILE_ID`, naming a profile in the developer's own installed app; the key
 *   is then read out of the Windows Credential Manager here, inside the WDIO process, so it never
 *   reaches a shell history, a log, or the spec's own output.
 *
 * Neither set means the live case reports BLOCKED rather than failing, which is the right outcome
 * for a machine with no provider account.
 */
export function readOnePieceApiKey() {
  const direct = process.env.VANEHUB_ONEPIECE_API_KEY?.trim();
  if (direct) {
    return direct;
  }
  const profileId = process.env.VANEHUB_ONEPIECE_PROFILE_ID?.trim();
  if (!profileId || process.platform !== "win32") {
    return null;
  }
  return readWindowsCredential(`onepiece-profile:${profileId}.vanehub-api-agent`);
}

function readWindowsCredential(target) {
  // keyring-rs stores the password as UTF-16LE bytes in CredentialBlob; base64 round-trips it
  // through PowerShell without the console code page mangling non-ASCII.
  const script = `
$ErrorActionPreference = 'Stop'
$signature = @"
using System;
using System.Runtime.InteropServices;
public class CredentialReader {
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct CREDENTIAL {
    public UInt32 Flags; public UInt32 Type; public IntPtr TargetName; public IntPtr Comment;
    public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
    public UInt32 CredentialBlobSize; public IntPtr CredentialBlob; public UInt32 Persist;
    public UInt32 AttributeCount; public IntPtr Attributes; public IntPtr TargetAlias; public IntPtr UserName;
  }
  [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern bool CredReadW(string target, uint type, uint flags, out IntPtr credential);
  [DllImport("advapi32.dll", SetLastError = true)]
  public static extern void CredFree(IntPtr buffer);
  public static string Read(string target) {
    IntPtr handle;
    if (!CredReadW(target, 1, 0, out handle)) { return null; }
    try {
      CREDENTIAL credential = (CREDENTIAL)Marshal.PtrToStructure(handle, typeof(CREDENTIAL));
      byte[] blob = new byte[credential.CredentialBlobSize];
      Marshal.Copy(credential.CredentialBlob, blob, 0, (int)credential.CredentialBlobSize);
      return Convert.ToBase64String(blob);
    } finally { CredFree(handle); }
  }
}
"@
Add-Type -TypeDefinition $signature -Language CSharp
$value = [CredentialReader]::Read(${JSON.stringify(target)})
if ($null -eq $value) { exit 3 }
Write-Output $value
`;
  let encoded;
  try {
    encoded = execFileSync(
      "powershell",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { encoding: "utf8", timeout: 30_000, windowsHide: true },
    ).trim();
  } catch {
    return null;
  }
  if (!encoded) {
    return null;
  }
  const secret = Buffer.from(encoded, "base64").toString("utf16le").replace(/\0+$/, "");
  return secret.length > 0 ? secret : null;
}
