export function watchSurfaceReadiness(root: HTMLElement, onReady: () => void): () => void {
  let active = true;
  const observer = new MutationObserver(() => completeWhenVisible());

  function completeWhenVisible() {
    if (!active || !root.firstElementChild) return;
    active = false;
    observer.disconnect();
    onReady();
  }

  observer.observe(root, { childList: true });
  completeWhenVisible();

  return () => {
    active = false;
    observer.disconnect();
  };
}
