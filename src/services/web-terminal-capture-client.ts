export type WebCaptureSource = "pty" | "quick-command" | "gap";
export interface WebCaptureChunk { id: number; sessionId: string; connectionId: string | null; terminalId: string | null; runId: string | null; source: WebCaptureSource; content: string; capturedAt: string; }

const chunks: WebCaptureChunk[] = [];
let nextId = 1;
let capacityBytes = 512 * 1024 * 1024;

export function configureWebCapture(capacity: number): void { capacityBytes = Math.max(1, capacity); }
export function captureWebOutput(input: Omit<WebCaptureChunk, "id" | "capturedAt">): WebCaptureChunk { const chunk = { ...input, id: nextId++, capturedAt: new Date().toISOString() }; chunks.push(chunk); while (chunks.reduce((total, item) => total + item.content.length, 0) > capacityBytes) chunks.shift(); return { ...chunk }; }
export function captureWebGap(sessionId: string): WebCaptureChunk { return captureWebOutput({ sessionId, connectionId: null, terminalId: null, runId: null, source: "gap", content: "[capture gap]" }); }
export function searchWebCapture(query: string, sessionId?: string, offset = 0, limit = 50): WebCaptureChunk[] { const normalized = query.toLocaleLowerCase(); return chunks.filter((chunk) => (!sessionId || chunk.sessionId === sessionId) && chunk.content.toLocaleLowerCase().includes(normalized)).slice(offset, offset + Math.min(Math.max(limit, 1), 100)).map((chunk) => ({ ...chunk })); }
export function purgeWebCapture(sessionId?: string): number { const before = chunks.length; if (sessionId) { for (let index = chunks.length - 1; index >= 0; index -= 1) if (chunks[index].sessionId === sessionId) chunks.splice(index, 1); } else chunks.length = 0; return before - chunks.length; }
