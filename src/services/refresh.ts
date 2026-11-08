import { listen } from "@tauri-apps/api/event";

// Collapse event bursts and allow one read at a time. A change received during
// a read schedules a trailing read, so the final update cannot be lost.
export async function subscribeChanges(
  event: string,
  changed: () => void | Promise<unknown>,
  project?: string,
): Promise<() => void> {
  let disposed = false;
  let pending = false;
  let running = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const schedule = () => {
    if (disposed || !pending || running || timer || document.hidden) return;
    timer = setTimeout(() => {
      timer = undefined;
      if (disposed || document.hidden) return;
      pending = false;
      running = true;
      void Promise.resolve()
        .then(changed)
        .catch(() => {}) // Subscribers report their own read errors in the UI.
        .finally(() => {
          running = false;
          schedule();
        });
    }, 100);
  };
  const visible = () => {
    if (!document.hidden) {
      pending = true;
      schedule();
    }
  };
  const stop = await listen<string | undefined>(event, ({ payload }) => {
    if (project && typeof payload === "string" && payload && payload !== project)
      return;
    pending = true;
    schedule();
  });
  document.addEventListener("visibilitychange", visible);
  return () => {
    disposed = true;
    if (timer) clearTimeout(timer);
    document.removeEventListener("visibilitychange", visible);
    stop();
  };
}
