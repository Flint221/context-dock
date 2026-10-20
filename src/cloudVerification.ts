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

export function hasCloudData(view: CloudView) {
  return (
    !!view.store_id ||
    view.retainedFiles > 0 ||
    ["pending", "creating", "linked", "deleted"].includes(view.store_state) ||
    view.jobs.length > 0
  );
}
export function cleanupPayload(view: CloudView) {
  return {
    confirm: true,
    confirmation: view.store_name ?? "",
    storeId: view.store_id,
  };
}
export function verifyCloudDeleted(view: CloudView) {
  if (
    view.enabled ||
    !view.paused ||
    view.store_id ||
    view.retainedFiles ||
    view.jobs.length ||
    view.store_state !== "none" ||
    view.status !== "cloud_deleted"
  ) {
    throw new Error(
      "The backend did not confirm cloud deletion. Refresh status and retry cleanup; cloud resources may remain.",
    );
  }
}
