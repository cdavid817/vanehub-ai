export function captureProtocolUrl(runId: string, displayToken: string): string {
  const path = `${encodeURIComponent(runId)}/${encodeURIComponent(displayToken)}`;
  return navigator.userAgent.includes("Windows")
    ? `http://vanehub-capture.localhost/${path}`
    : `vanehub-capture://localhost/${path}`;
}
