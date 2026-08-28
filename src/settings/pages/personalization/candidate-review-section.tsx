import { Inbox } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { MemoryCandidate } from "../../../types/personalization-memory";
import { SectionPanel } from "../page-parts";
import { CandidateEditor } from "./candidate-editor";
import { emptyEdits, reviewRequestFrom, useCandidateReview, type CandidateEdits } from "./use-candidate-review";
import { useScopeOptions } from "./use-scope-options";

/**
 * The queue of proposals waiting for a decision.
 *
 * Nothing here is stored yet. A proposal is text an Agent suggested, and the whole point of the
 * queue is that no automatic writer can put it into active memory -- only a person deciding can.
 */
export function CandidateReviewSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t, i18n } = useTranslation();
  const review = useCandidateReview(service);
  const { agents, workspaces } = useScopeOptions(service);
  const [openId, setOpenId] = useState<string | null>(null);
  const [edits, setEdits] = useState<CandidateEdits>(emptyEdits());

  function openEditor(candidate: MemoryCandidate) {
    setOpenId(candidate.id);
    setEdits({
      ...emptyEdits(),
      name: candidate.name ?? "",
      description: candidate.description ?? "",
      content: candidate.content ?? "",
      memoryType: candidate.memoryType === "untyped" || !candidate.memoryType ? "user" : candidate.memoryType,
    });
  }

  return (
    <SectionPanel
      description={t("personalization.review.description")}
      icon={Inbox}
      title={t("personalization.review.title")}
    >
      {review.loadError ? (
        <p className="rounded-md border p-3 text-sm ucd-status-danger" data-testid="personalization-review-error" role="alert">
          {t("personalization.review.loadFailed")}
        </p>
      ) : review.isLoading ? (
        <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>
      ) : review.candidates.length === 0 ? (
        <p className="text-sm text-muted-foreground" data-testid="personalization-review-empty">
          {t("personalization.review.empty")}
        </p>
      ) : (
        <ul className="grid gap-3" data-testid="personalization-review-list">
          {review.candidates.map((candidate) => (
            <li
              className="ucd-panel rounded-md p-3"
              data-testid={`personalization-candidate-${candidate.id}`}
              key={candidate.id}
            >
              <div className="flex flex-wrap items-baseline gap-2">
                <Badge tone="muted">{t(`personalization.review.kind.${candidate.kind}`)}</Badge>
                <span className="wrap-break-word text-sm font-medium">
                  {candidate.name ?? t("personalization.review.unnamed")}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`personalization.memoryList.source.${candidate.source}`)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {formatAppDateTime(candidate.createdAt, i18n.language, { dateStyle: "medium", timeStyle: "short" })}
                </span>
              </div>
              {candidate.content ? (
                <p className="mt-2 wrap-break-word whitespace-pre-wrap text-sm" data-testid={`personalization-candidate-content-preview-${candidate.id}`}>
                  {candidate.content}
                </p>
              ) : null}

              {review.conflictId === candidate.id ? (
                <p className="mt-2 rounded-md border p-2 text-xs ucd-status-warning" data-testid={`personalization-candidate-conflict-${candidate.id}`} role="alert">
                  {t("personalization.review.conflict")}
                </p>
              ) : null}
              {review.failedId === candidate.id ? (
                <p className="mt-2 text-xs ucd-status-danger" data-testid={`personalization-candidate-failed-${candidate.id}`} role="alert">
                  {t("personalization.review.failed")}
                </p>
              ) : null}

              {openId === candidate.id ? (
                <CandidateEditor
                  agents={agents}
                  candidate={candidate}
                  edits={edits}
                  onChange={(patch) => setEdits((current) => ({ ...current, ...patch }))}
                  workspaces={workspaces}
                />
              ) : null}

              <div className="mt-3 flex flex-wrap gap-2">
                {openId === candidate.id ? (
                  <>
                    <Button
                      data-testid={`personalization-candidate-approve-edits-${candidate.id}`}
                      disabled={review.isReviewing}
                      onClick={() =>
                        review.review(
                          reviewRequestFrom(candidate.id, edits, {
                            name: candidate.name,
                            description: candidate.description,
                            content: candidate.content,
                          }),
                        )
                      }
                      size="sm"
                    >
                      {t("personalization.review.approveWithEdits")}
                    </Button>
                    <Button
                      data-testid={`personalization-candidate-cancel-${candidate.id}`}
                      onClick={() => setOpenId(null)}
                      size="sm"
                      variant="outline"
                    >
                      {t("personalization.detail.cancel")}
                    </Button>
                  </>
                ) : (
                  <>
                    <Button
                      data-testid={`personalization-candidate-approve-${candidate.id}`}
                      disabled={review.isReviewing}
                      onClick={() => review.review({ candidateId: candidate.id, action: "approve" })}
                      size="sm"
                    >
                      {t("personalization.review.approve")}
                    </Button>
                    <Button
                      data-testid={`personalization-candidate-edit-${candidate.id}`}
                      disabled={review.isReviewing}
                      onClick={() => openEditor(candidate)}
                      size="sm"
                      variant="outline"
                    >
                      {t("personalization.review.edit")}
                    </Button>
                    {candidate.targetId ? (
                      <Button
                        data-testid={`personalization-candidate-merge-${candidate.id}`}
                        disabled={review.isReviewing || candidate.expectedTargetRevision === null}
                        onClick={() =>
                          review.review({
                            candidateId: candidate.id,
                            action: "merge-into",
                            mergeTargetId: candidate.targetId ?? undefined,
                            // The revision the proposal was written against. Merging without it
                            // would fold this text over an edit made since, unseen.
                            mergeExpectedRevision: candidate.expectedTargetRevision ?? undefined,
                          })
                        }
                        size="sm"
                        variant="outline"
                      >
                        {t("personalization.review.merge")}
                      </Button>
                    ) : null}
                    <Button
                      data-testid={`personalization-candidate-sensitive-${candidate.id}`}
                      disabled={review.isReviewing}
                      onClick={() =>
                        review.review({ candidateId: candidate.id, action: "mark-sensitive-and-archive" })
                      }
                      size="sm"
                      variant="outline"
                    >
                      {t("personalization.review.markSensitive")}
                    </Button>
                    <Button
                      data-testid={`personalization-candidate-reject-${candidate.id}`}
                      disabled={review.isReviewing}
                      onClick={() => review.review({ candidateId: candidate.id, action: "reject" })}
                      size="sm"
                      variant="outline"
                    >
                      {t("personalization.review.reject")}
                    </Button>
                  </>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </SectionPanel>
  );
}
