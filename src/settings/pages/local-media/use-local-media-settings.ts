import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { localMediaErrorCodeFrom } from "../../../session-workspace/local-media/local-media-errors";
import { localMediaService } from "../../../services/runtime-local-media-client";
import type {
  AudioDeviceCatalog,
  LocalMediaEngine,
  LocalMediaErrorCode,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  ProfileFieldIssue,
} from "../../../types/local-media";

/** Poll interval for a probe. Probes are short and rare, so a timer beats an event subscription. */
const PROBE_POLL_MS = 400;
const PROBE_TIMEOUT_MS = 120_000;

export type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "conflict" }
  | { kind: "failed"; code: LocalMediaErrorCode };

export interface LocalMediaSettingsModel {
  draft: LocalMediaProfile | null;
  devices: AudioDeviceCatalog;
  dirty: boolean;
  issues: Map<string, ProfileFieldIssue>;
  loadError: LocalMediaErrorCode | null;
  loading: boolean;
  nativeAvailable: boolean;
  probing: LocalMediaEngine | null;
  saveState: SaveState;
  status: LocalMediaRuntimeStatus | null;
  discard: () => void;
  probe: (engine: LocalMediaEngine) => void;
  reloadFromNative: () => void;
  save: () => void;
  update: (mutate: (draft: LocalMediaProfile) => LocalMediaProfile) => void;
}

export function issueKeyFor(engine: LocalMediaEngine | null, field: string): string {
  return `${engine ?? "profile"}:${field}`;
}

function indexIssues(issues: ProfileFieldIssue[]): Map<string, ProfileFieldIssue> {
  return new Map(issues.map((issue) => [issueKeyFor(issue.engine, issue.field), issue]));
}

const EMPTY_DEVICES: AudioDeviceCatalog = { inputs: [], outputs: [] };

/**
 * Everything the Local media page needs, and nothing a component would otherwise reimplement.
 *
 * Two rules drive the shape. Probes run against the *saved* profile, never the draft, because a
 * probe starts a real worker with real model paths and answering "is this configured correctly"
 * for text the user has not committed would report readiness for a configuration that does not
 * exist. And the draft is never overwritten by a background refresh: a status poll landing mid-edit
 * must not discard what the user typed.
 */
export function useLocalMediaSettings(isActive: boolean): LocalMediaSettingsModel {
  const [saved, setSaved] = useState<LocalMediaProfile | null>(null);
  const [draft, setDraft] = useState<LocalMediaProfile | null>(null);
  const [status, setStatus] = useState<LocalMediaRuntimeStatus | null>(null);
  const [devices, setDevices] = useState<AudioDeviceCatalog>(EMPTY_DEVICES);
  const [issues, setIssues] = useState<Map<string, ProfileFieldIssue>>(new Map());
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<LocalMediaErrorCode | null>(null);
  const [nativeAvailable, setNativeAvailable] = useState(false);
  const [probing, setProbing] = useState<LocalMediaEngine | null>(null);
  const [saveState, setSaveState] = useState<SaveState>({ kind: "idle" });

  // Unmount can happen while a probe is still polling; without this the poll would keep calling
  // setState on a dead component and, worse, keep a worker's result alive for nobody.
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [available, profile, runtimeStatus] = await Promise.all([
        localMediaService.isAvailable(),
        localMediaService.getProfile(),
        localMediaService.getStatus(),
      ]);
      if (!alive.current) return;
      setNativeAvailable(available);
      setSaved(profile);
      setDraft((current) => current ?? profile);
      setStatus(runtimeStatus);
      setLoadError(null);
      // Device enumeration touches the audio host and can fail on its own; an empty catalog is a
      // usable page (the system default still works) whereas a failed load is not.
      const catalog = await localMediaService.listAudioDevices().catch(() => EMPTY_DEVICES);
      if (alive.current) setDevices(catalog);
    } catch (error) {
      if (alive.current) setLoadError(localMediaErrorCodeFrom(error));
    } finally {
      if (alive.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!isActive) return;
    void load();
  }, [isActive, load]);

  const update = useCallback((mutate: (current: LocalMediaProfile) => LocalMediaProfile) => {
    setSaveState({ kind: "idle" });
    setDraft((current) => (current ? mutate(current) : current));
  }, []);

  const dirty = useMemo(
    () => Boolean(draft && saved && JSON.stringify(draft) !== JSON.stringify(saved)),
    [draft, saved],
  );

  const discard = useCallback(() => {
    setDraft(saved);
    setIssues(new Map());
    setSaveState({ kind: "idle" });
  }, [saved]);

  const save = useCallback(() => {
    if (!draft) return;
    setSaveState({ kind: "saving" });
    void (async () => {
      try {
        // Native validation is authoritative and runs first, so a rejected field is reported
        // against its input rather than as one opaque code from the save.
        const found = await localMediaService.validateProfile(draft);
        if (!alive.current) return;
        setIssues(indexIssues(found));
        if (found.length > 0) {
          setSaveState({ kind: "failed", code: found[0].code });
          return;
        }
        const stored = await localMediaService.saveProfile({
          profile: draft,
          expectedRevision: draft.revision,
        });
        if (!alive.current) return;
        setSaved(stored);
        setDraft(stored);
        setSaveState({ kind: "saved" });
        setStatus(await localMediaService.getStatus());
      } catch (error) {
        if (!alive.current) return;
        const code = localMediaErrorCodeFrom(error);
        setSaveState(code === "PROFILE_REVISION_CONFLICT" ? { kind: "conflict" } : { kind: "failed", code });
      }
    })();
  }, [draft]);

  const probe = useCallback((engine: LocalMediaEngine) => {
    setProbing(engine);
    void (async () => {
      try {
        const handle = await localMediaService.probeEngine(engine);
        const deadline = Date.now() + PROBE_TIMEOUT_MS;
        for (;;) {
          if (!alive.current) return;
          const result = await localMediaService.getOperationResult(handle.operationId);
          if (result?.kind === "probe") {
            if (alive.current) setStatus(result.result);
            return;
          }
          if (Date.now() >= deadline) {
            // Give up polling but leave the operation alone: the supervisor owns its lifetime and
            // will time it out on its own terms.
            return;
          }
          await new Promise((resolve) => setTimeout(resolve, PROBE_POLL_MS));
        }
      } catch (error) {
        if (alive.current) setSaveState({ kind: "failed", code: localMediaErrorCodeFrom(error) });
      } finally {
        if (alive.current) setProbing(null);
      }
    })();
  }, []);

  const reloadFromNative = useCallback(() => {
    setDraft(null);
    setIssues(new Map());
    setSaveState({ kind: "idle" });
    void load();
  }, [load]);

  return {
    draft,
    devices,
    dirty,
    issues,
    loadError,
    loading,
    nativeAvailable,
    probing,
    saveState,
    status,
    discard,
    probe,
    reloadFromNative,
    save,
    update,
  };
}
