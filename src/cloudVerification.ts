import type { CloudView, Draft } from "./cloud";
export function verifyCloudDraft(view: CloudView, draft: Draft) {
  const selected = view.families
    .filter((f) => f.selected)
    .map((f) => f.id)
    .sort();
  if (
    !!view.enabled !== draft.enabled ||
    !!view.auto_sync !== draft.automatic ||
    view.mode !== draft.mode ||
    view.soft_limit !== draft.softLimit ||
    JSON.stringify(selected) !== JSON.stringify([...draft.documents].sort())
  )
    throw new Error(
      "The backend did not confirm the requested cloud settings. Refresh status and try again.",
    );
}
export function verifyCloudAction(view: CloudView, action: string) {
  const failed =
    action === "disconnect"
      ? !!view.enabled || !view.paused
      : action === "stop_automatic"
        ? !!view.auto_sync || !view.paused
        : action === "pause"
          ? !view.paused
          : ["start", "sync", "resume"].includes(action)
            ? !view.enabled ||
              !!view.paused ||
              (action === "start" && !view.started)
            : false;
  if (failed)
    throw new Error(
      "The backend did not confirm the requested cloud action. Refresh status and try again.",
    );
}
