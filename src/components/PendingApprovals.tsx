import { useEffect, useState } from "react";
import { Check, FolderOpen, RefreshCw } from "lucide-react";
import {
  desktop,
  type PendingFile,
  type ReviewRequest,
  type ReviewPlan,
} from "../services/desktop";
import { registry } from "../services/registry";
import { Modal } from "./Modal";
import { useUnsavedChanges } from "../draftGuard";
export function PendingApprovals({
  project,
  openProject,
}: {
  project: string | null;
  openProject: (id: string) => void;
}) {
  const [files, setFiles] = useState<PendingFile[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [review, setReview] = useState<{
    request: ReviewRequest;
    plan: ReviewPlan;
  } | null>(null);
  const guard = useUnsavedChanges();
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    const refresh = () =>
      void desktop
        .pending()
        .then((next) => {
          if (active) {
            setFiles(next);
            setSelected((prev) =>
              prev.filter((key) =>
                next.some((f) => key === `${f.projectId}:${f.path}`),
              ),
            );
            setLoading(false);
          }
        })
        .catch((e) => {
          if (active) {
            setError(String(e));
            setLoading(false);
          }
        });
    refresh();
    desktop
      .subscribe(refresh)
      .then((s) => {
        if (active) stop = s;
        else s();
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  const groups = Array.from(new Set(files.map((f) => f.projectId))).sort(
    (a, b) => (a === project ? -1 : b === project ? 1 : 0),
  );
  async function reload() {
    const next = await desktop.pending();
    setFiles(next);
    setSelected((prev) =>
      prev.filter((key) =>
        next.some((f) => key === `${f.projectId}:${f.path}`),
      ),
    );
    setLoading(false);
  }
  async function preview(id: string, action: ReviewRequest["action"]) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const paths = files
        .filter(
          (f) => f.projectId === id && selected.includes(`${id}:${f.path}`),
        )
        .map((f) => f.path);
      const request = { project: id, paths, action };
      setReview({ request, plan: await desktop.preview(request) });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function apply() {
    if (!review) return;
    setBusy(true);
    setError("");
    try {
      await desktop.apply(review.request, review.plan.token);
      setReview(null);
      setSelected([]);
      setNotice(
        "Review saved. Local files and history are retained; the project’s tracking and cloud settings still apply.",
      );
      await reload();
    } catch (e) {
      setReview(null);
      setError(String(e));
      await reload().catch(() => {});
    } finally {
      setBusy(false);
    }
  }
  useEffect(
    () =>
      guard.register("pending-review", {
        dirty: selected.length > 0,
        saving: busy,
        save: async () => {
          setError(
            "Confirm Approve selected or Ignore selected before leaving, or discard the selection.",
          );
          return false;
        },
        discard: () => {
          setSelected([]);
          setReview(null);
        },
      }),
    [guard, selected, busy],
  );
  return (
    <div className="pending-inbox" aria-label="Pending approvals">
      <div className="banner">
        <span>
          Nothing uploads just because it appears here. Select files and confirm
          approval. Ignoring keeps them excluded; you can include them later in
          the folder tree.
        </span>
        <button
          disabled={busy}
          onClick={() => void reload().catch((e) => setError(String(e)))}
        >
          <RefreshCw size={16} />
          Refresh inbox
        </button>
      </div>
      {error && (
        <p role="alert" className="error-text">
          {error}
        </p>
      )}
      {notice && (
        <div role="status" className="banner success">
          <Check size={16} />
          {notice}
        </div>
      )}
      {loading ? (
        <p>Loading pending files…</p>
      ) : groups.length === 0 ? (
        <section className="panel empty">
          <h2>No files awaiting approval</h2>
          <p>
            Approved document edits and recognized versions continue under each
            project’s saved settings.
          </p>
        </section>
      ) : (
        groups.map((id) => {
          const rows = files.filter((f) => f.projectId === id);
          const count = rows.filter((f) =>
            selected.includes(`${id}:${f.path}`),
          ).length;
          return (
            <section
              className="panel approval-group"
              key={id}
              aria-label={rows[0].projectName}
              data-highlighted={id === project}
            >
              <div className="approval-heading">
                <div>
                  <h2>{rows[0].projectName}</h2>
                  <p>
                    {rows.length} awaiting review · {count} selected
                  </p>
                </div>
                <button
                  disabled={busy}
                  onClick={() => guard.request(() => openProject(id))}
                >
                  View project
                </button>
                <button
                  disabled={busy}
                  onClick={() =>
                    void registry.open(id).catch((e) => setError(String(e)))
                  }
                >
                  <FolderOpen size={16} />
                  Open folder
                </button>
              </div>
              <div className="approval-table">
                <table>
                  <thead>
                    <tr>
                      <th>Select</th>
                      <th>File / relative path</th>
                      <th>Type</th>
                      <th>Size</th>
                      <th>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((f) => (
                      <tr key={f.path}>
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`Select ${f.path}`}
                            checked={selected.includes(`${id}:${f.path}`)}
                            disabled={busy}
                            onChange={(e) =>
                              setSelected((prev) =>
                                e.target.checked
                                  ? [...prev, `${id}:${f.path}`]
                                  : prev.filter((p) => p !== `${id}:${f.path}`),
                              )
                            }
                          />
                        </td>
                        <td>{f.path}</td>
                        <td>{f.kind}</td>
                        <td>{(f.size / 1024).toFixed(1)} KiB</td>
                        <td>
                          <span className={`badge ${f.status}`}>
                            {f.status}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="toolbar-actions">
                <button
                  disabled={
                    busy ||
                    count === 0 ||
                    rows.some(
                      (f) =>
                        selected.includes(`${id}:${f.path}`) &&
                        f.status !== "available",
                    )
                  }
                  className="primary"
                  onClick={() => void preview(id, "approve")}
                >
                  Approve selected ({count})
                </button>
                <button
                  disabled={busy || count === 0}
                  onClick={() => void preview(id, "ignore")}
                >
                  Ignore selected
                </button>
                <button
                  disabled={busy}
                  onClick={() =>
                    setSelected((prev) => [
                      ...prev.filter((p) => !p.startsWith(`${id}:`)),
                      ...rows
                        .filter((f) => f.status === "available")
                        .map((f) => `${id}:${f.path}`),
                    ])
                  }
                >
                  Select available files
                </button>
                <button
                  disabled={busy || count === 0}
                  onClick={() =>
                    setSelected((prev) =>
                      prev.filter((p) => !p.startsWith(`${id}:`)),
                    )
                  }
                >
                  Clear selection
                </button>
              </div>
            </section>
          );
        })
      )}
      {review && (
        <Modal
          title={
            review.request.action === "approve"
              ? "Approve new files?"
              : "Ignore new files?"
          }
          close={() => {
            if (!busy) setReview(null);
          }}
        >
          <p>
            {review.plan.projectName}: {review.plan.paths.length} selected
            files.
          </p>
          <ul className="review-paths">
            {review.plan.paths.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
          <p>
            {review.request.action === "ignore"
              ? "These files will stay excluded from tracking and cloud indexing. Source files and existing local history are kept."
              : `These paths become selected. Tracking is ${review.plan.trackingEnabled ? "on" : "off"}; cloud sync is ${review.plan.cloudEnabled ? "on" : "off"}. Approval does not enable either setting. Uploads follow the saved project policy and any global pause.`}
          </p>
          <div className="actions">
            <button disabled={busy} onClick={() => setReview(null)}>
              Cancel
            </button>
            <button
              disabled={busy}
              className="primary"
              onClick={() => void apply()}
            >
              {busy ? "Saving…" : "Confirm and save"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
