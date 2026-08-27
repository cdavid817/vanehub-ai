/**
 * Route navigation that survives the application restoring its previous destination.
 *
 * Pushing the route and trusting it is not enough. The application resumes the destination it was
 * last on, and that restore reads persisted state over IPC, so it can land *after* a push that
 * followed `data-vanehub-bootstrap === "ready"` and silently replace it. On the Windows and Linux
 * runners the restore happens to win the race early and nothing is noticed; on the slower macOS
 * runner it landed late, leaving the sweep on the settings page it had just finished capturing
 * while it waited thirty seconds for a session tab bar that was never going to appear.
 *
 * So the push is repeated until the location stays where it was put.
 */
export async function navigateTo(path) {
  const settle = async () => {
    await globalThis.browser.execute((target) => {
      globalThis.history.pushState({}, "", target);
      globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
    }, path);
    // Long enough for a restore already in flight to land and be observed, rather than to land
    // just after the check and be missed.
    await globalThis.browser.pause(400);
    const current = decodeURIComponent(await globalThis.browser.execute(() => globalThis.location.pathname));
    const requested = decodeURIComponent(path);
    // A deeper path counts as arrived: the workspace canonicalises `/workspace/sessions` to
    // `/workspace/sessions/<active id>` by design. What this rejects is landing somewhere else
    // entirely, which is what a late restore does.
    return current === requested || current.startsWith(`${requested}/`);
  };

  await globalThis.browser.waitUntil(settle, {
    timeout: 30_000,
    interval: 100,
    timeoutMsg: `The application would not stay on ${path}; something kept navigating away from it.`,
  });
}
