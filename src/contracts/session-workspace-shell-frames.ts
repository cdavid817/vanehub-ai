import { z } from "zod";

export const shellStreamSchema = z.enum(["pty", "stdout", "stderr", "system"]);

export const sessionShellStateSchema = z.enum([
  "starting",
  "opening",
  "running",
  "closing",
  "reaping",
  "close_failed",
  "exited",
  "disconnected",
  "failed",
  "closed",
]);

export const shellCloseDispositionSchema = z.enum([
  "closed",
  "reaping",
  "close_failed",
  "already_terminal",
]);

/**
 * What one close attempt achieved.
 *
 * `finalState` is optional at the schema level for the same reason it is optional in the domain: an
 * unsettled close observed nothing final, and defaulting it here would manufacture the exact claim
 * the disposition exists to withhold.
 */
export const shellCloseOutcomeSchema = z.object({
  shellId: z.string().min(1),
  generation: z.number().int().nonnegative(),
  disposition: shellCloseDispositionSchema,
  finalState: sessionShellStateSchema.optional(),
  reason: z.string().nullish().transform((value) => value ?? undefined),
  retryable: z.boolean(),
  attempt: z.number().int().nonnegative(),
  cleanupDeadlineReached: z.boolean(),
});

export const shellForegroundProcessStateSchema = z.enum(["present", "absent", "unknown"]);

export const shellOutputFrameSchema = z.object({
  shellId: z.string().min(1),
  sequence: z.number().int().nonnegative(),
  occurredAt: z.string(),
  stream: shellStreamSchema,
  data: z.string(),
});

export const shellReplayGapSchema = z.object({
  fromSequence: z.number().int().nonnegative(),
  toSequence: z.number().int().nonnegative(),
  reason: z.string(),
});

/**
 * The runtime descriptor is parsed by `normalizeShellRuntimeDescriptor`, which derives capabilities
 * from the kind rather than trusting them from the wire. This schema therefore validates the
 * descriptor's presence and leaves its interpretation to that one place.
 */
export const sessionShellDescriptorSchema = z.object({
  shellId: z.string().min(1),
  generation: z.number().int().nonnegative(),
  sessionId: z.string().min(1),
  seatId: z.string().min(1).optional(),
  title: z.string(),
  runtime: z.unknown(),
  state: sessionShellStateSchema,
  reason: z.string().optional(),
  exitCode: z.number().int().optional(),
  createdAt: z.string(),
  lastActivityAt: z.string(),
  revision: z.number().int().nonnegative(),
  foregroundProcess: shellForegroundProcessStateSchema,
});

export const shellAttachSnapshotSchema = z.object({
  attachmentId: z.string().min(1),
  descriptor: sessionShellDescriptorSchema,
  replay: z.array(shellOutputFrameSchema),
  nextSequence: z.number().int().nonnegative(),
  gap: shellReplayGapSchema.optional(),
});

export const sessionShellStateNoticeSchema = z.object({
  shellId: z.string().min(1),
  generation: z.number().int().nonnegative(),
  sessionId: z.string().min(1),
  state: sessionShellStateSchema,
  reason: z.string().nullish(),
  exitCode: z.number().int().nullish(),
  revision: z.number().int().nonnegative(),
  occurredAt: z.string(),
});

/**
 * The event payload, discriminated on `type`. Output and state are separate shapes because they
 * carry different facts: a frame has a sequence and a state change has a revision, and a single
 * flattened shape would make every reader check which half of it is meaningful.
 */
export const sessionShellNoticeSchema = z.discriminatedUnion("type", [
  shellOutputFrameSchema.extend({
    type: z.literal("output"),
    sessionId: z.string().min(1),
  }),
  sessionShellStateNoticeSchema.extend({ type: z.literal("state") }),
]);
