import { useMutation } from "@tanstack/react-query";
import { FilePenLine, LockKeyhole } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorCandidateDetail } from "../../../types/skill-curator";

export function SkillCuratorDraftEditor({ detail, onChanged, service }: {
  detail: CuratorCandidateDetail;
  onChanged: () => Promise<unknown>;
  service: SkillCuratorService;
}) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<"learned_guidance" | "exact_patch">("learned_guidance");
  const [guidance, setGuidance] = useState("");
  const [oldString, setOldString] = useState("");
  const [newString, setNewString] = useState("");
  const [replaceAll, setReplaceAll] = useState(false);
  const [rationale, setRationale] = useState("");
  const [expected, setExpected] = useState("");
  const mutation = useMutation({
    mutationFn: () => service.saveSkillCuratorDraft({
      schemaVersion: 1,
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      idempotencyKey: actionKey("draft", detail.candidateId),
      mutation: kind === "learned_guidance"
        ? { kind, guidance }
        : { kind, oldString, newString, replaceAll },
      rationale,
      expectedEffectiveChange: expected,
    }),
    onSuccess: async (result) => { if (result.ok) await onChanged(); },
  });
  const allowed = ["awaiting_draft", "ready_for_review", "apply_failed"].includes(detail.state);
  const error = mutation.data && !mutation.data.ok ? mutation.data.error : undefined;
  const bodyValid = kind === "learned_guidance" ? guidance.trim() : oldString.length > 0 && newString.length > 0;
  return <section className="rounded-xl border border-border bg-background p-4" aria-labelledby="curator-draft-title">
    <div className="flex items-start gap-2"><FilePenLine className="mt-0.5 h-4 w-4 text-primary" /><div><h4 className="text-sm font-semibold" id="curator-draft-title">{t("skills.curator.editor.title")}</h4><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.curator.editor.description")}</p></div></div>
    <div className="mt-3 flex gap-2" role="radiogroup" aria-label={t("skills.curator.editor.kind")}><KindButton active={kind === "learned_guidance"} onClick={() => setKind("learned_guidance")} text={t("skills.curator.value.learn_block")} /><KindButton active={kind === "exact_patch"} onClick={() => setKind("exact_patch")} text={t("skills.curator.value.exact_patch")} /></div>
    <div className="mt-3 space-y-3">{kind === "learned_guidance" ? <TextArea label={t("skills.curator.editor.guidance")} onChange={setGuidance} value={guidance} /> : <><TextArea label={t("skills.curator.editor.oldString")} onChange={setOldString} value={oldString} /><TextArea label={t("skills.curator.editor.newString")} onChange={setNewString} value={newString} /><label className="flex items-center gap-2 text-xs"><input checked={replaceAll} className="h-4 w-4 accent-primary" onChange={(event) => setReplaceAll(event.target.checked)} type="checkbox" />{t("skills.curator.editor.replaceAll")}</label></>}
      <TextArea label={t("skills.curator.editor.rationale")} maxLength={2048} onChange={setRationale} value={rationale} />
      <TextArea label={t("skills.curator.editor.expected")} maxLength={2048} onChange={setExpected} value={expected} />
    </div>
    <div className="mt-3 flex gap-2 rounded-md border border-warning/30 bg-warning/5 p-3 text-xs leading-5 text-muted-foreground"><LockKeyhole className="mt-0.5 h-4 w-4 shrink-0 text-warning" /><span>{t("skills.curator.editor.prohibited")}</span></div>
    {error ? <p className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">{t("skills.curator.editor.error", { code: error.reasonCode ?? error.code })}</p> : null}
    {!allowed ? <p className="mt-3 text-xs text-warning" role="status">{t("skills.curator.editor.unavailable", { state: detail.state })}</p> : null}
    <div className="mt-4 flex justify-end"><Button disabled={!allowed || !bodyValid || !rationale.trim() || !expected.trim() || mutation.isPending} onClick={() => mutation.mutate()}>{mutation.isPending ? t("skills.curator.editor.saving") : t("skills.curator.editor.save")}</Button></div>
  </section>;
}

export function actionKey(action: string, candidateId: string) {
  return `${action}-${candidateId}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function KindButton({ active, onClick, text }: { active: boolean; onClick: () => void; text: string }) {
  return <button aria-checked={active} className={`rounded-md border px-3 py-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${active ? "border-primary bg-primary text-primary-foreground" : "border-border"}`} onClick={onClick} role="radio" type="button">{text}</button>;
}

function TextArea({ label, maxLength = 8192, onChange, value }: { label: string; maxLength?: number; onChange: (value: string) => void; value: string }) {
  return <label className="block text-xs text-muted-foreground"><span>{label}</span><textarea className="mt-1 min-h-24 w-full resize-y rounded-md border border-border bg-background p-2 font-mono text-xs text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" maxLength={maxLength} onChange={(event) => onChange(event.target.value)} value={value} /></label>;
}
