import { useCallback, useEffect, useRef, useState } from "react";
import { registry } from "../services/registry";
import { drive, type DriveProject } from "../drive";
import type {
  Project,
  TrackingView,
  Family,
  PathSelectionView,
} from "../types";
import { beginOperation } from "../operations";
import { date } from "../format";

export function LatestDocuments({
  project,
  changed,
  selection,
}: {
  project: Project;
  changed: () => Promise<void>;
  selection: PathSelectionView | null;
}) {
  const [view, setView] = useState<TrackingView | null>(null);
  const [cloud, setCloud] = useState<DriveProject | null>(null);
  const [cloudUnknown, setCloudUnknown] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [targets, setTargets] = useState<Record<string, string>>({});
  const [query, setQuery] = useState("");
  const alive = useRef(true);
  const sequence = useRef(0);
  const refresh = useCallback(async () => {
    const current = ++sequence.current;
    const [tracking, publication] = await Promise.allSettled([
      registry.tracking(project.id),
      drive.project(project.id),
    ]);
    if (!alive.current || current !== sequence.current) return;
    if (tracking.status === "fulfilled" && tracking.value)
      setView(tracking.value);
    else if (tracking.status === "rejected") setError(String(tracking.reason));
    setCloudUnknown(publication.status === "rejected" || !publication.value);
    if (publication.status === "fulfilled") setCloud(publication.value ?? null);
  }, [project.id]);
  useEffect(() => {
    alive.current = true;
    const stops: (() => void)[] = [];
    const keep = (stop: () => void) => {
      if (alive.current) stops.push(stop);
      else stop();
    };
    void registry
      .subscribe(refresh)
      .then(keep)
      .catch((e) => {
        if (alive.current) setError(String(e));
      });
    void drive
      .subscribe(project.id, refresh)
      .then(keep)
      .catch(() => {});
    return () => {
      alive.current = false;
      stops.forEach((s) => s());
    };
  }, [project.id, refresh]);
  useEffect(() => {
    void Promise.resolve()
      .then(refresh)
      .catch((e) => {
        if (alive.current) setError(String(e));
      });
  }, [
    refresh,
    project.lastScan,
    project.trackingRevision,
    project.folderState,
  ]);
  async function run(action: () => Promise<unknown>) {
    const release = beginOperation(`latest-documents:${project.id}`);
    if (!release) return;
    setBusy(true);
    setError("");
    try {
      await action();
      await refresh();
      await changed();
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      release();
      if (alive.current) setBusy(false);
    }
  }
  const selected = (family: Family) => {
    const source = view?.sources.find((s) => s.id === family.activeSourceId);
    if (!source || !selection) return true;
    const path = source.relativePath.replaceAll("\\", "/").toLowerCase();
    return !selection.exclusions.some(
      (r) =>
        path === r.path.toLowerCase() ||
        (r.kind === "folder" &&
          (!r.path || path.startsWith(`${r.path.toLowerCase()}/`))),
    );
  };
  const families =
    view?.families.filter(
      (f) =>
        f.state !== "grouped_history" &&
        f.state !== "merged" &&
        f.state !== "ignored" &&
        selected(f) &&
        `${f.name} ${f.activeFilename ?? ""}`
          .toLowerCase()
          .includes(query.toLowerCase()),
    ) ?? [];
  function status(family: Family) {
    if (family.needsReview) return "Needs attention";
    if (family.sourceStatus !== "available") return "Waiting for file";
    if (!view?.enabled) return "Tracking paused";
    if (cloudUnknown) return "Needs attention";
    if (!cloud?.enabled) return "Local only";
    if (!cloud.connected) return "Offline";
    const remote = cloud.documents.find((d) => d.documentId === family.id);
    if (remote?.state === "needs_attention") return "Needs attention";
    if (remote?.state === "offline") return "Offline";
    if (remote?.state === "waiting_file") return "Waiting for file";
    if (remote?.state === "uploading" || remote?.state === "verifying")
      return "Uploading";
    if (remote?.state === "synced" && remote.verifiedHash && remote.verifiedAt)
      return "Up to date";
    return "Queued";
  }
  return (
    <section className="panel" aria-labelledby="latest-documents-heading">
      <div className="section-heading">
        <div>
          <h2 id="latest-documents-heading">
            Latest documents{" "}
            <span className="count">{view?.enabled ? families.length : 0}</span>
          </h2>
          <p>
            One document per version family. Your newest working file stays in
            its project folder.
          </p>
        </div>
        <label className="check-row">
          <input
            type="checkbox"
            aria-label="Monitor selected files"
            checked={view?.enabled ?? false}
            disabled={busy || !view}
            onChange={(e) => {
              const enabled = e.target.checked;
              void run(async () => {
                await registry.configureTracking(project.id, enabled, false);
                const saved = await registry.tracking(project.id);
                if (saved.enabled !== enabled)
                  throw new Error(
                    "The tracking change did not save. Refresh and retry.",
                  );
              });
            }}
          />
          Monitor selected files
        </label>
      </div>
      {!view && <p role="status">Loading latest documents…</p>}
      {view && !view.enabled && (
        <p>
          Choose the files and folders below, then enable monitoring to group
          their versions and follow changes.
        </p>
      )}
      {view?.enabled && families.length === 0 && (
        <p>
          No selected documents are ready yet. Check the folder selection or add
          a Markdown or Word file.
        </p>
      )}
      {view?.enabled && (
        <label>
          Find a document
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            aria-label="Find a latest document"
            placeholder="Search document names"
          />
        </label>
      )}
      {view?.enabled && families.length > 0 && (
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Latest document</th>
                <th>Version</th>
                <th>Status</th>
                <th>Last verified in Drive</th>
                <th>Location</th>
              </tr>
            </thead>
            <tbody>
              {families.slice(0, 200).map((family) => {
                const remote = cloud?.documents.find(
                  (d) => d.documentId === family.id,
                );
                const label = status(family);
                const source = view.sources.find(
                  (s) => s.id === family.activeSourceId,
                );
                return (
                  <tr key={family.id}>
                    <td>
                      <strong>{family.activeFilename || family.name}</strong>
                      {source && (
                        <small className="full-path">
                          {source.relativePath}
                        </small>
                      )}
                      {remote?.error && <p role="alert">{remote.error}</p>}
                    </td>
                    <td>{family.currentLabel || "Working"}</td>
                    <td>
                      <span
                        className={`badge ${label === "Up to date" ? "available" : label === "Needs attention" ? "missing" : ""}`}
                      >
                        {label}
                      </span>
                    </td>
                    <td>
                      {remote?.verifiedAt ? date(remote.verifiedAt) : "—"}
                    </td>
                    <td>
                      <button
                        disabled={busy || family.sourceStatus !== "available"}
                        onClick={() =>
                          void run(() => registry.openCurrent(family.id))
                        }
                      >
                        Show file
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {families.length > 200 && (
            <p>
              Showing the first 200 document families. Narrow the folder
              selection to show fewer files.
            </p>
          )}
        </div>
      )}
      {!!view?.reviews.length && (
        <details open>
          <summary>{view.reviews.length} file matches need attention</summary>
          {view.reviews.map((review) => (
            <article className="review-card" key={review.sourceId}>
              <strong>{review.relativePath}</strong>
              <p>{review.explanation}</p>
              <label>
                Document family
                <select
                  value={targets[review.sourceId] ?? ""}
                  aria-label={`Document family for ${review.name}`}
                  disabled={busy}
                  onChange={(e) =>
                    setTargets((previous) => ({
                      ...previous,
                      [review.sourceId]: e.target.value,
                    }))
                  }
                >
                  <option value="">Choose a family…</option>
                  {families
                    .filter((f) => f.state === "tracked")
                    .map((f) => (
                      <option key={f.id} value={f.id}>
                        {f.name}
                      </option>
                    ))}
                </select>
              </label>
              <button
                disabled={busy || !targets[review.sourceId]}
                onClick={() =>
                  void run(() =>
                    registry.resolve(
                      project.id,
                      review.sourceId,
                      "assign",
                      targets[review.sourceId],
                    ),
                  )
                }
              >
                Use this family
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  void run(() =>
                    registry.resolve(project.id, review.sourceId, "new"),
                  )
                }
              >
                Keep as a separate document
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  void run(() =>
                    registry.resolve(project.id, review.sourceId, "ignore"),
                  )
                }
              >
                Ignore this file
              </button>
            </article>
          ))}
        </details>
      )}
      {view?.error && <p role="alert">{view.error}</p>}
      {cloudUnknown && (
        <p role="alert">
          Drive status is unavailable. Local file monitoring continues.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {busy && <p role="status">Updating document tracking…</p>}
    </section>
  );
}
