import { useEffect, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type { ExpertRole } from "../types/expert-role";
import type { KnownProject, KnownRemoteWorkspace } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";

/**
 * The lists the create-session dialog offers to choose from.
 *
 * Separated from the dialog's form state because the two have nothing to do with each other: these
 * are read once when the dialog opens and never edited, while the form is edited constantly and
 * never read from a service. Keeping them together meant one effect both loaded reference data and
 * reset user input, so a refreshed Agent list silently cleared a half-filled form.
 *
 * Every load degrades to an empty list rather than failing the dialog: a session can still be
 * created without a recent-projects shortcut, and an unusable dialog is a worse answer than a
 * shorter menu.
 */
export function useCreateSessionReferenceData(open: boolean) {
  const [expertRoles, setExpertRoles] = useState<ExpertRole[]>([]);
  const [knownProjects, setKnownProjects] = useState<KnownProject[]>([]);
  const [knownRemoteWorkspaces, setKnownRemoteWorkspaces] = useState<KnownRemoteWorkspace[]>([]);
  const [sshConnections, setSshConnections] = useState<SshConnection[]>([]);

  useEffect(() => {
    if (!open) return;
    void agentService.listExpertRoles().then(setExpertRoles).catch(() => setExpertRoles([]));
    void agentService.listKnownProjects().then(setKnownProjects).catch(() => setKnownProjects([]));
    void agentService
      .listKnownRemoteWorkspaces()
      .then(setKnownRemoteWorkspaces)
      .catch(() => setKnownRemoteWorkspaces([]));
    void sshConnectionService
      .listConnections()
      .then(setSshConnections)
      .catch(() => setSshConnections([]));
  }, [open]);

  return { expertRoles, knownProjects, knownRemoteWorkspaces, sshConnections };
}
