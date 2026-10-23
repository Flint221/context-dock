import type { PathSelectionView, TrackingView } from "../types";
import type { CloudView } from "../cloud";
const normalized = (path: string) => path.replaceAll("\\", "/").toLowerCase();
export function ProjectSummary({
  selection,
  tracking,
  cloud,
  unavailable,
}: {
  selection: PathSelectionView | null;
  tracking: TrackingView | null;
  cloud: CloudView | null;
  unavailable: boolean;
}) {
  const selected = selection?.entries.filter(
    (e) =>
      e.kind === "file" &&
      !selection.exclusions.some(
        (r) =>
          normalized(r.path) === normalized(e.path) ||
          (r.kind === "folder" &&
            (!r.path ||
              normalized(e.path).startsWith(`${normalized(r.path)}/`))),
      ),
  );
  const selectedPaths = new Set(selected?.map((e) => normalized(e.path)));
  const trackedIds = new Set(
    tracking?.sources
      .filter((s) => selectedPaths.has(normalized(s.relativePath)))
      .map((s) => s.documentId),
  );
  const tracked = tracking?.families.filter(
    (f) =>
      trackedIds.has(f.id) &&
      f.versionCount > 0 &&
      f.state === "tracked" &&
      f.sourceStatus === "available" &&
      !f.needsReview,
  ).length;
  const total = cloud
    ? Math.max(
        cloud.indexed + cloud.pending + cloud.failed,
        cloud.families.filter((f) => f.selected && f.eligible).length,
      )
    : 0;
  const failed =
    !!cloud &&
    (!!cloud.failed ||
      !!cloud.error ||
      !!cloud.localError ||
      ["cleanup_pending", "cleanup_failed"].includes(cloud.status));
  const enabled = !!cloud?.enabled && !cloud.paused;
  const complete =
    enabled &&
    total > 0 &&
    cloud?.indexed === total &&
    !cloud.pending &&
    cloud.connectionStatus === "connected" &&
    cloud.localReady !== false &&
    !cloud.families.some(
      (f) =>
        !f.pathExcluded &&
        (f.versionCount === 0 ||
          (f.selected && (!f.eligible || f.currentNumber !== f.cloudNumber))),
    );
  const syncTone =
    unavailable || failed
      ? "error"
      : !cloud || !enabled
        ? "neutral"
        : complete
          ? "success"
          : "warning";
  const syncLabel = unavailable
    ? "Folder unavailable"
    : !cloud
      ? "Status unavailable"
      : failed
        ? "Needs attention"
        : !enabled
          ? cloud.paused
            ? "Paused"
            : "Cloud off"
          : complete
            ? "Indexed"
            : "Indexing / waiting";
  const missing = selected?.filter((e) => e.status !== "available").length ?? 0;
  const attention =
    missing +
    (tracking?.reviews.length ?? 0) +
    (tracking?.operations.filter((o) => o.error).length ?? 0);
  return (
    <section className="panel project-summary" aria-label="Project overview">
      <div>
        <span>Selected files</span>
        <strong>{selection ? selected?.length : "—"}</strong>
        <small>Saved folder choices</small>
      </div>
      <div>
        <span>Tracked documents</span>
        <strong>
          {unavailable || !tracking || !selection ? "—" : tracked}
        </strong>
        <small>
          {unavailable
            ? "Folder unavailable"
            : !tracking
              ? "Status unavailable"
              : tracking.enabled
                ? "Tracking on · local snapshots"
                : "Tracking off · history retained"}
        </small>
      </div>
      <div className={`summary-${syncTone}`}>
        <span>Cloud indexed</span>
        <strong>{cloud ? `${cloud.indexed}/${total}` : "—"}</strong>
        <small>{syncLabel}</small>
        {enabled && total > 0 && (
          <progress
            aria-label="Cloud indexing coverage"
            value={cloud!.indexed}
            max={total}
          />
        )}
      </div>
      <div
        className={
          unavailable || attention ? "summary-error" : "summary-neutral"
        }
      >
        <span>Needs attention</span>
        <strong>
          {unavailable
            ? "Folder lost"
            : !tracking || !selection
              ? "—"
              : attention}
        </strong>
        <small>
          {unavailable
            ? "Reconnect or reassign"
            : `${missing} unavailable files · ${tracking?.reviews.length ?? 0} to review`}
        </small>
      </div>
    </section>
  );
}
