import { useEffect, useRef, useState } from "react";
import { Archive, BookOpen, Check, Eye, RefreshCw } from "lucide-react";
import type { Project, TrackingView, PendingMatch, Family } from "../types";
import { registry } from "../services/registry";
import { date } from "../format";
import { Modal } from "./Modal";
import { DocumentHistory } from "./DocumentHistory";
import { beginOperation } from "../operations";
import { OperationStatus } from "./OperationStatus";
function MatchDecision({
  review,
  families,
  busy,
  decide,
}: {
  review: PendingMatch;
  families: Family[];
  busy: boolean;
  decide: (action: "assign" | "new" | "ignore", target?: string) => void;
}) {
  const [target, setTarget] = useState("");
  return (
    <article className="review-card">
      <strong>{review.relativePath}</strong>
      <p>{review.explanation}</p>
      <label>
        Assign to document family
        <select
          value={target}
          disabled={busy}
          onChange={(e) => setTarget(e.target.value)}
        >
          <option value="">Choose a family…</option>
          {families
            .filter((f) => f.versionCount > 0 && f.state === "tracked")
            .map((f) => (
              <option key={f.id} value={f.id}>
                {f.name} — {f.activeFilename}
                {review.candidateIds.includes(f.id) ? " (suggested)" : ""}
              </option>
            ))}
        </select>
      </label>
      <div className="toolbar-actions">
        <button
          disabled={busy || !target}
          onClick={() => decide("assign", target)}
        >
          <Check size={14} />
          Assign as version
        </button>
        <button disabled={busy} onClick={() => decide("new")}>
          New independent document
        </button>
        <button disabled={busy} onClick={() => decide("ignore")}>
          Ignore for tracking
        </button>
      </div>
    </article>
  );
}
export function TrackingPanel({
  project,
  changed,
}: {
  project: Project;
  changed: () => Promise<void>;
}) {
  const [view, setView] = useState<TrackingView | null>(null);
  const [stateKnown, setStateKnown] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [confirmation, setConfirmation] = useState<{
    enabled: boolean;
    archive: boolean;
    kind: "tracking" | "archival";
  } | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const [initialize, setInitialize] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [archive, setArchive] = useState(false);
  const [history, setHistory] = useState<string | null>(null);
  const [limit, setLimit] = useState(100);
  const [choiceLimit, setChoiceLimit] = useState(100);
  useEffect(() => {
    let active = true;
    registry
      .tracking(project.id)
      .then((v) => {
        if (active) {
          setView(v);
          setStateKnown(true);
        }
      })
      .catch((e) => {
        if (active) {
          setError(String(e));
          setStateKnown(false);
        }
      });
    return () => {
      active = false;
    };
  }, [
    project.id,
    project.lastScan,
    project.trackingEnabled,
    project.archiveEnabled,
    project.trackingRevision,
  ]);
  async function mutate(
    work: () => Promise<unknown>,
    expected?: { enabled: boolean; archive: boolean },
    done?: () => void,
  ) {
    const release = beginOperation(`tracking:${project.id}`);
    if (!release) return;
    setBusy(true);
    setError("");
    setNotice("");
    if (expected) setStateKnown(false);
    try {
      await work();
      const next = await registry.tracking(project.id);
      if (alive.current) {
        setView(next);
        setStateKnown(true);
      }
      if (
        expected &&
        (next.enabled !== expected.enabled ||
          next.archiveEnabled !== expected.archive)
      )
        throw new Error(
          "The backend did not confirm the requested tracking settings. Refresh documents and try again.",
        );
      await changed();
      if (alive.current) {
        done?.();
        if (expected)
          setNotice(
            "Tracking settings saved and confirmed." +
              (next.error
                ? ` Some document operations need attention: ${next.error}`
                : ""),
          );
      }
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      release();
      if (alive.current) setBusy(false);
    }
  }
  const eligible =
    view?.sources.filter(
      (s) => s.status === "available" && s.disposition !== "superseded",
    ) || [];
  const family = view?.families.find((f) => f.id === history);
  const captured =
    view?.families.filter(
      (f) => f.versionCount > 0 && f.state !== "grouped_history",
    ) || [];
  const retainedHistories =
    view?.families.filter((f) => f.state === "grouped_history") || [];
  return (
    <section className="panel tracking-panel">
      <div className="section-heading">
        <div>
          <h2>
            <Eye size={18} />
            Document tracking
          </h2>
          <p>
            Discovery reads metadata. Tracking saves immutable local snapshots.
            Archival organizes replaced sources.
          </p>
        </div>
        {view && (
          <span className={`badge ${view.enabled ? view.watchStatus : ""}`}>
            {view.enabled ? view.watchStatus : "Tracking off"}
          </span>
        )}
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {view?.importNotice && (
        <p className="banner warning" role="status">
          {view.importNotice}
        </p>
      )}
      <OperationStatus
        busy={busy}
        message="Applying and checking tracking settings…"
        success={notice}
      />
      {view && !stateKnown && !busy && (
        <p role="status" className="tracking-note">
          Tracking status could not be verified. Use Scan for changes to reload
          it before changing settings.
        </p>
      )}
      {view === null ? (
        <p className="padded">Loading tracking settings…</p>
      ) : (
        <>
          <div className="tracking-controls">
            <div>
              <strong>Automatic tracking</strong>
              <p>
                Runs while ContextDock is open; changes made while closed are
                reconciled on launch or resume.
              </p>
            </div>
            <button
              disabled={busy || !!confirmation || initialize || !stateKnown}
              aria-busy={busy}
              className={view.enabled ? "" : "primary"}
              onClick={() => {
                if (view.enabled || view.initialized)
                  setConfirmation({
                    enabled: !view.enabled,
                    archive: view.archiveEnabled,
                    kind: "tracking",
                  });
                else {
                  setSelected(eligible.map((s) => s.id));
                  setArchive(false);
                  setChoiceLimit(100);
                  setInitialize(true);
                }
              }}
            >
              {busy
                ? "Working…"
                : view.enabled
                  ? "Disable tracking"
                  : view.initialized
                    ? "Resume tracking"
                    : "Enable tracking…"}
            </button>
          </div>
          <label className="tracking-toggle">
            <input
              type="checkbox"
              checked={view.archiveEnabled}
              disabled={
                busy ||
                !!confirmation ||
                initialize ||
                !view.enabled ||
                !stateKnown
              }
              onChange={(e) =>
                setConfirmation({
                  enabled: view.enabled,
                  archive: e.target.checked,
                  kind: "archival",
                })
              }
            />
            <span>
              <strong>
                Enable automatic archival of replaced source files
              </strong>
              <small>
                Separately opt in to Old Context Files. A confirmed replacement
                can move the previous source after both snapshots are verified.
                In-place edits archive a copy of the old snapshot and leave the
                current source untouched.
              </small>
            </span>
          </label>
          {!view.enabled && view.initialized && (
            <p className="tracking-note">
              Tracking is paused. Your history is retained. Resume catches up
              with changes; pending archives run only if archival is also
              enabled.
            </p>
          )}
          {view.error && (
            <div role="alert" className="banner error">
              <span>{view.error}</span>
            </div>
          )}
          <div className="toolbar-actions">
            <button
              disabled={busy}
              onClick={() => void mutate(() => registry.refresh(project.id))}
            >
              <RefreshCw size={15} />
              Scan for changes
            </button>
          </div>
          {view.reviews.length > 0 && (
            <div className="review-section">
              <h3>
                Needs Review{" "}
                <span className="count">{view.reviews.length}</span>
              </h3>
              <p>
                These files will not be moved while a decision is pending.
                Assignments are remembered for future matching within the same
                folder.
              </p>
              {view.reviews.slice(0, 100).map((review) => (
                <MatchDecision
                  key={review.sourceId}
                  review={review}
                  families={view.families}
                  busy={busy || !view.enabled}
                  decide={(action, target) =>
                    void mutate(() =>
                      registry.resolve(
                        project.id,
                        review.sourceId,
                        action,
                        target || null,
                      ),
                    )
                  }
                />
              ))}
              {view.reviews.length > 100 && (
                <p>Resolve the first 100 to see additional pending files.</p>
              )}
            </div>
          )}
          <div className="family-section">
            <h3>
              <BookOpen size={17} />
              Document families <span className="count">{captured.length}</span>
            </h3>
            {captured.length === 0 ? (
              <p className="tracking-note">
                No captured versions yet. Enable tracking and select files to
                import their document histories.
              </p>
            ) : (
              <>
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>Document family / active source</th>
                        <th>Current</th>
                        <th>Versions</th>
                        <th>Last capture</th>
                        <th>Status</th>
                        <th>History</th>
                      </tr>
                    </thead>
                    <tbody>
                      {captured.slice(0, limit).map((f) => (
                        <tr key={f.id}>
                          <td>
                            <strong>{f.name}</strong>
                            <small>
                              {f.activeFilename || "No active source"}
                            </small>
                          </td>
                          <td>
                            {f.currentNumber ? `v${f.currentNumber}` : "—"}
                          </td>
                          <td>{f.versionCount}</td>
                          <td>{date(f.lastCaptured)}</td>
                          <td>
                            <span
                              className={`badge ${f.needsReview ? "warning" : !view.enabled ? "paused" : f.sourceStatus || ""}`}
                            >
                              {f.needsReview
                                ? "Needs Review"
                                : view.enabled
                                  ? f.sourceStatus
                                  : "Paused"}
                            </span>
                          </td>
                          <td>
                            <button onClick={() => setHistory(f.id)}>
                              View history
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {captured.length > limit && (
                  <button onClick={() => setLimit((n) => n + 100)}>
                    Show 100 more families
                  </button>
                )}
              </>
            )}
          </div>
          {retainedHistories.length > 0 && (
            <details className="family-section">
              <summary>
                Earlier separate histories ({retainedHistories.length})
              </summary>
              <p className="tracking-note">
                These original snapshots were retained when numbered files were
                grouped. Their IDs and history are unchanged. Regrouped families
                need explicit selection again in Advanced cloud settings.
              </p>
              {retainedHistories.slice(0, limit).map((f) => (
                <div className="toolbar-actions" key={f.id}>
                  <span>
                    {f.activeFilename || f.name} · {f.versionCount} retained
                    version(s)
                  </span>
                  <button
                    onClick={() => setHistory(f.id)}
                    aria-label={`View earlier history of ${f.activeFilename || f.name}`}
                  >
                    View earlier history
                  </button>
                </div>
              ))}
              {retainedHistories.length > limit && (
                <button onClick={() => setLimit((n) => n + 100)}>
                  Show 100 more earlier histories
                </button>
              )}
            </details>
          )}
          {view.enabled &&
            view.sources.some(
              (s) =>
                s.status === "available" &&
                (s.disposition === "ignored" || s.disposition === "discovered"),
            ) && (
              <details className="untracked">
                <summary>Files not being tracked</summary>
                <p>
                  These files remain available for discovery. Start an
                  independent history when you choose.
                </p>
                {view.sources
                  .filter(
                    (s) =>
                      s.status === "available" &&
                      (s.disposition === "ignored" ||
                        s.disposition === "discovered"),
                  )
                  .slice(0, limit)
                  .map((s) => (
                    <div className="untracked-row" key={s.id}>
                      <span>{s.relativePath}</span>
                      <button
                        disabled={busy}
                        onClick={() =>
                          void mutate(() =>
                            registry.resolve(project.id, s.id, "new"),
                          )
                        }
                      >
                        Track independently
                      </button>
                    </div>
                  ))}
              </details>
            )}
          {view.operations.length > 0 && (
            <div className="operation-section">
              <h3>
                <Archive size={17} />
                Pending operations{" "}
                <span className="count">{view.operations.length}</span>
              </h3>
              <p>
                Failed or interrupted work is retained for safe retry. Existing
                files are never overwritten. Archives pause when archival is
                off.
              </p>
              {view.operations.map((op) => (
                <div className="operation" key={op.id}>
                  <strong>
                    {op.kind} · {op.state}
                  </strong>
                  <p className={op.error ? "error" : undefined}>
                    {op.error || "Awaiting completion or project permission."}
                  </p>
                  <small>{date(op.createdAt)}</small>
                </div>
              ))}
              <button
                disabled={busy || !view.enabled}
                onClick={() => void mutate(() => registry.retry(project.id))}
              >
                Retry pending operations
              </button>
            </div>
          )}
          {view.activity.length > 0 && (
            <details className="tracking-activity">
              <summary>
                Recent tracking activity ({view.activity.length})
              </summary>
              <ol>
                {view.activity.map((event) => (
                  <li key={event.id}>
                    <span>{event.message}</span>
                    <time>{date(event.createdAt)}</time>
                  </li>
                ))}
              </ol>
            </details>
          )}
        </>
      )}
      {initialize && view && (
        <Modal
          title="Initialize document tracking"
          close={() => {
            if (!busy) setInitialize(false);
          }}
        >
          <p>
            Select files to track. Clear numbered sequences such as
            Report_v1.md, Report_v2.md and Report_v10.md in the same folder and
            file type become one document family, imported in numeric order with
            the newest export active. Unrelated or ambiguous names stay
            separate. Initialization never moves your original files.
          </p>
          <p className="tracking-note">
            Unselected existing files are ignored for tracking until you include
            them and Save changes in the folder tree. New unrelated files added
            later are tracked automatically unless their folder is excluded.
          </p>
          <div className="selection-actions">
            <button
              disabled={busy}
              onClick={() => setSelected(eligible.map((s) => s.id))}
            >
              Select all
            </button>
            <button disabled={busy} onClick={() => setSelected([])}>
              Clear selection
            </button>
            <span>{selected.length} selected</span>
          </div>
          <div className="source-selection">
            {eligible.slice(0, choiceLimit).map((s) => (
              <label key={s.id} className="tracking-toggle">
                <input
                  type="checkbox"
                  disabled={busy}
                  checked={selected.includes(s.id)}
                  onChange={(e) =>
                    setSelected((ids) =>
                      e.target.checked
                        ? [...ids, s.id]
                        : ids.filter((id) => id !== s.id),
                    )
                  }
                />
                <span>{s.relativePath}</span>
              </label>
            ))}
            {eligible.length > choiceLimit && (
              <button onClick={() => setChoiceLimit((n) => n + 100)}>
                Show 100 more files
              </button>
            )}
            {eligible.length === 0 && (
              <p>
                No available supported files. Tracking can wait for new
                documents.
              </p>
            )}
          </div>
          <label className="tracking-toggle">
            <input
              type="checkbox"
              disabled={busy}
              checked={archive}
              onChange={(e) => setArchive(e.target.checked)}
            />
            <span>
              Also enable automatic archival
              <small>
                Future confirmed replacements may move previous sources into Old
                Context Files. Initial sources remain in place.
              </small>
            </span>
          </label>
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
          <div className="actions">
            <button disabled={busy} onClick={() => setInitialize(false)}>
              Cancel
            </button>
            <button
              disabled={busy}
              className="primary"
              onClick={() =>
                void mutate(
                  () =>
                    registry.configureTracking(
                      project.id,
                      true,
                      archive,
                      selected,
                    ),
                  { enabled: true, archive },
                  () => setInitialize(false),
                )
              }
            >
              {busy
                ? "Capturing selected files…"
                : "Capture selected and enable"}
            </button>
          </div>
        </Modal>
      )}
      {family && (
        <DocumentHistory
          family={family}
          project={project.id}
          close={() => setHistory(null)}
        />
      )}
      {confirmation && (
        <Modal
          title={
            confirmation.kind === "archival"
              ? confirmation.archive
                ? "Enable automatic archival?"
                : "Turn off automatic archival?"
              : confirmation.enabled
                ? "Resume document tracking?"
                : "Turn off document tracking?"
          }
          close={() => {
            if (!busy) {
              setConfirmation(null);
              setError("");
            }
          }}
        >
          <p>
            {confirmation.kind === "archival"
              ? confirmation.archive
                ? "Future replacements can move older sources into Old Context Files after verification. In-place edits archive a copy of the previous version. Your current document stays available."
                : "Stop creating archives or moving replaced source files. Version tracking and existing archives are kept."
              : confirmation.enabled
                ? "Resume watching this folder and capture changes made while tracking was off. Clear numbered exports saved as separate initial histories by older versions can be grouped when no cloud resources remain. Original histories stay available under Earlier separate histories; regrouped families require explicit cloud re-selection."
                : "Stop capturing new document versions. Existing history, saved chats and documents are kept. Cloud sync will not receive uncaptured edits."}
          </p>
          {error && <p role="alert">{error}</p>}
          <OperationStatus
            busy={busy}
            message="Saving and checking the backend state…"
          />
          <div className="toolbar-actions">
            <button
              className="primary"
              disabled={busy}
              aria-busy={busy}
              onClick={() =>
                void mutate(
                  () =>
                    registry.configureTracking(
                      project.id,
                      confirmation.enabled,
                      confirmation.archive,
                    ),
                  confirmation,
                  () => setConfirmation(null),
                )
              }
            >
              {busy
                ? "Saving…"
                : confirmation.kind === "archival"
                  ? confirmation.archive
                    ? "Enable archival"
                    : "Turn off archival"
                  : confirmation.enabled
                    ? "Resume tracking"
                    : "Turn off tracking"}
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setConfirmation(null);
                setError("");
              }}
            >
              Cancel
            </button>
          </div>
        </Modal>
      )}
    </section>
  );
}
