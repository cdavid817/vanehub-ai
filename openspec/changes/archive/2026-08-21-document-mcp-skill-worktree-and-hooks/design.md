## Context

The guides reached chapter-level completeness in `complete-english-user-guide-content` and gained a coverage requirement in `complete-chinese-user-guide-coverage`. That requirement is satisfied by a capability having a chapter or a named section — which turns out to be necessary but not sufficient. A capability can hold a chapter and still not say what it is.

See `proposal.md` for the measured gap between spec material and guide coverage per capability, and for the worktree coverage violation.

## Goals / Non-Goals

**Goals:**

- For each of the five capabilities, state what it is, what it is made of, and what it can and cannot do.
- Put every claim on a spec requirement or a rendered interface label, not on plausibility.
- Keep both locales structurally equivalent.

**Non-Goals:**

- Auditing the remaining chapters for the same depth. Five were named; a systematic depth audit is its own piece of work, and pretending this covers it would repeat the mistake that let the worktree gap survive.
- Adding a requirement about coverage *depth*. It would be hard to state without becoming unfalsifiable, and the existing requirement plus review is what caught this.
- New screenshots. The two moved from `tooling.md` are reused; no new capture is introduced.
- Changing product behavior, labels, or specs.

## Decisions

### D1: Three new chapters, not three bigger sections

**Choice:** MCP, Prompt Hook, and Git worktree get their own chapters; Skill and Loop Engineering are expanded in place.

The split follows where the material lives. MCP alone has 25 requirements — folding that into `tooling.md` would produce a chapter dominated by one subject while its neighbours get a paragraph each. Skill and Loop already had substantial chapters that were missing an opening, not a body.

Git worktree had no owner at all. It is referenced from `first-session.md`, `core-concepts.md`, `loop-engineering.md`, and `use-cases.md`, each assuming the reader already knows what a worktree is. A shared concept referenced from four chapters is exactly what a chapter is for.

**`tooling.md`'s sections become pointers rather than being deleted**, so a reader who arrives at the settings-oriented chapter still finds the subject and one hop to the detail.

### D2: Answer "what is it" by contrast, not by definition alone

**Choice:** Each opening says what the thing is *not*, against the neighbour it is most confused with.

A Skill is not custom instructions (which describe *you*, globally) and not MCP (**MCP supplies tools, a Skill supplies method**). A Prompt Hook is not a prompt editor — what you change is the slot every assembly passes through, not one conversation's input. A worktree is not `git clone` — it shares one history. A Loop is not free multi-Agent collaboration — the runtime holds the advance.

**Why:** a bare definition is easy to nod along to and hard to act on. The confusion each of these actually produces is with a specific neighbour, and naming that neighbour is what makes the definition load-bearing.

### D3: Surface the constraint that changes a decision

Each chapter leads its "notes" with the fact that would change what a reader does, rather than with the most technically interesting one.

For MCP that is plaintext storage of environment variables and headers — it decides whether you put a long-lived credential there, and nothing had said it. For a Loop it is that the Verifier is read-only, because it explains why "review passed" means anything. For a worktree it is that a Loop's worktree is never cleaned up, because directories accumulate silently otherwise.

### D4: Map VaneHub onto the topology taxonomy row by row, including the empty rows

**Choice:** The seven-topology table marks VaneHub's position in every row, including the four where the answer is "not implemented" or "deliberately not used".

The empty rows carry the information. A table showing only what exists reads like a feature list; one that says Parallel, Hierarchical, and Market are not implemented, and Supervisor is deliberately not used, tells a reader what the product is *not* trying to be.

**Two mappings needed care rather than a tick:**

- **Group chat is a hybrid.** It shares a message pool like a blackboard but has no turn scheduler — the speaking Agent hands off. AutoGen's GroupChat has a manager that picks the next speaker; this does not. Marking the row a plain "yes" would have been wrong.
- **Loop is Sequential/Pipeline.** Fixed phase order, fixed role split, advanced statically by the runtime. This is the substantive reason the two mechanisms share no orchestration logic — they are not in the same row — which is a better explanation than the chapter's previous assertion that they simply do not share code.

## Risks / Trade-offs

- **[The guides grew by roughly 30k characters, which is more to keep current]** → accepted; the alternative was chapters that name controls without explaining them, which drifts just as fast and is harder to notice.
- **[Three new chapters change navigation for existing readers]** → both `SUMMARY.md` files and both `index.md` tables are updated in the same change, and the equivalence check covers all 32 chapters.
- **[Depth is not mechanically checkable]** → true, and the reason D-non-goals declines to add a depth requirement. Chapter presence is checkable and already required; depth stays a review property.
- **[The topology taxonomy is industry context, not product behavior]** → it is presented as a classification with VaneHub's position marked per row, so the product claims stay separable from the taxonomy.
- **[Five more label mismatches suggest there are others]** → likely. Every one so far was found by writing prose against `en.json` and `zh-CN.json` rather than by any check, which is a gap a future change could close with a lint over documented labels.
