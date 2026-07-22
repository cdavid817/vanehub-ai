export type SshAuthMode = "password" | "key";
export type SshConnectionTestStatus = "not-tested" | "succeeded" | "failed";

export interface SshConnection {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  defaultPath: string;
  authMode: SshAuthMode;
  keyPath: string | null;
  hasPassword: boolean;
  testStatus: SshConnectionTestStatus;
  lastConnectedAt: string | null;
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SaveSshConnectionInput {
  name: string;
  host: string;
  port: number;
  user: string;
  defaultPath: string;
  authMode: SshAuthMode;
  keyPath?: string | null;
  password?: string | null;
}

export interface SshConnectionTestResult {
  status: SshConnectionTestStatus;
  message: string;
  testedAt: string;
}
