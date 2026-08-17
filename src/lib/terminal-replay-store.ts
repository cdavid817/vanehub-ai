import { BoundedTextBuffer } from "./bounded-text-buffer";

export class TerminalReplayStore {
  private readonly entries = new Map<string, BoundedTextBuffer>();
  private retainedBytes = 0;

  constructor(
    private readonly perSessionBytes: number,
    private readonly totalBytes: number,
  ) {}

  read(sessionId: string) {
    const entry = this.entries.get(sessionId);
    if (!entry) return "";
    this.touch(sessionId, entry);
    return entry.snapshot();
  }

  append(sessionId: string, content: string) {
    let entry = this.entries.get(sessionId);
    if (!entry) {
      entry = new BoundedTextBuffer(this.perSessionBytes);
      this.entries.set(sessionId, entry);
    }
    const previousBytes = entry.byteLength;
    entry.append(content);
    this.retainedBytes += entry.byteLength - previousBytes;
    this.touch(sessionId, entry);
    this.evictToLimit();
  }

  clear(sessionId: string) {
    const entry = this.entries.get(sessionId);
    if (!entry) return;
    this.retainedBytes -= entry.byteLength;
    this.entries.delete(sessionId);
  }

  private touch(sessionId: string, entry: BoundedTextBuffer) {
    this.entries.delete(sessionId);
    this.entries.set(sessionId, entry);
  }

  private evictToLimit() {
    while (this.retainedBytes > this.totalBytes) {
      const oldest = this.entries.entries().next().value as
        | [string, BoundedTextBuffer]
        | undefined;
      if (!oldest) return;
      this.retainedBytes -= oldest[1].byteLength;
      this.entries.delete(oldest[0]);
    }
  }
}
