import { z } from "zod";

export const shellStreamSchema = z.enum(["pty", "stdout", "stderr", "system"]);

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
  sessionId: z.string().min(1),
  seatId: z.string().min(1).optional(),
  title: z.string(),
  runtime: z.unknown(),
  state: z.enum(["connecting", "connected", "disconnected", "failed"]),
  createdAt: z.string(),
  lastActivityAt: z.string(),
});

export const shellAttachSnapshotSchema = z.object({
  descriptor: sessionShellDescriptorSchema,
  replay: z.array(shellOutputFrameSchema),
  nextSequence: z.number().int().nonnegative(),
  gap: shellReplayGapSchema.optional(),
});
