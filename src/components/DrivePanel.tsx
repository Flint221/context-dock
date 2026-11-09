import { useCallback, useEffect, useRef, useState } from "react";
import { drive, type DriveProject } from "../drive";
import { beginOperation } from "../operations";
import { date } from "../format";

export function DrivePanel({ project }: { project: string }) {
  const [view, setView] = useState<DriveProject | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [pausing, setPausing] = useState(false);
  const [cleanupBusy, setCleanupBusy] = useState(false);
  const alive = useRef(true);
  const refresh = useCallback(async () => {
    const next = await drive.project(project);
    if (alive.current) setView(next);
    return next;
  }, [project]);
  useEffect(() => {
    alive.current = true;
    let stop: (() => void) | undefined;
    void refresh().catch((e) => {
      if (alive.current) setError(String(e));
    });
    void drive
      .subscribe(project, async () => {
        await refresh();
      })
      .then((release) => {
        if (alive.current) stop = release;
        else release();
      })
      .catch((e) => {
        if (alive.current) setError(String(e));
      });
    return () => {
      alive.current = false;
      stop?.();
    };
  }, [project, refresh]);
  async function run(
    task: () => Promise<void>,
    success: string,
    verify?: (next: DriveProject) => boolean,
  ) {
    const release = beginOperation(`drive-publication:${project}`);
    if (!release) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await task();
      const next = await refresh();
      if (verify && !verify(next))
        throw new Error(
          "Drive did not confirm the requested publication state. Review status and retry.",
        );
      if (alive.current) setMessage(success);
    } catch (e) {
      if (alive.current) setError(String(e));
      try {
        await refresh();
      } catch {
        /* Preserve the actionable failure. */
      }
    } finally {
      release();
      if (alive.current) setBusy(false);
    }
  }
  async function pause() {
    const release = beginOperation(`drive-pause:${project}`);
    if (!release) return;
    setPausing(true);
    setError("");
    setMessage("");
    try {
      await drive.configure(project, false, view?.automatic ?? true);
      const next = await refresh();
      if (next.enabled)
        throw new Error("The publication pause did not save. Retry pausing.");
      if (alive.current)
        setMessage(
          "Drive publication paused. Any in-flight request will finish safely; no new upload will start.",
        );
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      release();
      if (alive.current) setPausing(false);
    }
  }
  async function setRecycling(recycle: boolean) {
    const release = beginOperation(`drive-recycling:${project}`);
    if (!release) return;
    setCleanupBusy(true);
    setError("");
    setMessage("");
    try {
      await drive.recycling(project, recycle);
      const next = await refresh();
      if (next.recycle !== recycle)
        throw new Error(
          "The recycling preference did not save. Refresh and retry.",
        );
      if (alive.current)
        setMessage(
          recycle
            ? "Older-file recycling enabled. A fresh verified publication is required before cleanup."
            : "Older-file recycling disabled. Your files are kept.",
        );
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      release();
      if (alive.current) setCleanupBusy(false);
    }
  }
  return (
    <section
      className="panel cloud-panel"
      aria-labelledby="drive-publication-heading"
    >
      <h2 id="drive-publication-heading">Google Drive publication</h2>
      <p>
        Keep one cloud file for each selected document. New versions replace its
        contents and filename while keeping its Drive link.
      </p>
      {view && (
        <>
          {view.enabled && !view.rootReady && (
            <div className="banner warning">
              <p>
                Choose your existing ContextDock folder in Settings, or create
                the Drive folders here if you do not already have one.
              </p>
              <button
                disabled={busy || !view.connected}
                onClick={() =>
                  void run(
                    () => drive.createFolders(project),
                    "Drive folders are ready. No documents were uploaded.",
                    (next) => next.rootReady && !!next.folderId,
                  )
                }
              >
                Create Drive folders
              </button>
            </div>
          )}
          {!view.connected && (
            <p>
              Connect the project's Google Drive account in Settings to publish.
              Your local files remain available.
            </p>
          )}
          <label className="check-row">
            <input
              type="checkbox"
              checked={view.enabled}
              disabled={busy || (!view.connected && !view.enabled)}
              onChange={(e) => {
                const enabled = e.target.checked;
                void run(
                  () => drive.configure(project, enabled, true),
                  enabled
                    ? "Drive publication enabled. Stable saves will publish automatically once the Drive folders are ready."
                    : "Drive publication paused. Cloud files are kept.",
                  (next) => next.enabled === enabled,
                );
              }}
            />
            Publish this project to Google Drive
          </label>
          <div className="cloud-actions">
            {view.enabled && !view.automatic && (
              <button
                disabled={busy || !view.connected}
                onClick={() =>
                  void run(
                    () => drive.configure(project, true, true),
                    "Automatic publication enabled. Stable saves will publish after settling.",
                    (next) => next.enabled && next.automatic,
                  )
                }
              >
                Enable automatic publication
              </button>
            )}
            {view.enabled && (
              <button disabled={pausing} onClick={() => void pause()}>
                Pause publication
              </button>
            )}
            <button
              className="primary"
              disabled={
                busy || !view.enabled || !view.connected || !view.rootReady
              }
              onClick={() =>
                void run(
                  () => drive.publish(project),
                  "Latest files published and verified.",
                  (next) =>
                    next.documents.length > 0 &&
                    next.documents.every((d) => d.state === "synced"),
                )
              }
            >
              Publish latest
            </button>
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await refresh();
                }, "Drive status refreshed.")
              }
            >
              Refresh publication status
            </button>
          </div>
          {view.documents.length > 0 && (
            <ul>
              {view.documents.map((document) => (
                <li key={document.documentId}>
                  <strong>
                    {document.pendingName ||
                      document.verifiedName ||
                      "Waiting for file"}
                  </strong>{" "}
                  · {document.state}
                  {document.verifiedAt && (
                    <> · Verified {date(document.verifiedAt)}</>
                  )}
                  {document.error && <p role="alert">{document.error}</p>}
                </li>
              ))}
            </ul>
          )}
          {view.error && !error && <p role="alert">{view.error}</p>}
          <details>
            <summary>Optional local cleanup</summary>
            <label className="check-row">
              <input
                type="checkbox"
                checked={view.recycle}
                disabled={
                  cleanupBusy ||
                  (!view.recycle && (busy || !view.connected || !view.enabled))
                }
                onChange={(e) => {
                  void setRecycling(e.target.checked);
                }}
              />
              Recycle unchanged older local versions
            </label>
            <p>
              Off by default. After a fresh Drive readback, clearly numbered
              older versions of the same family can go to the Windows Recycle
              Bin. The newest local working file stays in place. Changed or
              ambiguous files are kept. Automatic recycling supports local fixed
              NTFS drives.
            </p>
            <p>
              Cleanup uses a guarded safety folder inside .contextdock/recycle.
              Interrupted moves are restored when safe. Restoring from the
              Windows Recycle Bin returns a file to that safety folder; move it
              back to its original project location when needed.
            </p>
            {view.cleanupError && (
              <p role="alert">
                Older-file cleanup needs attention: {view.cleanupError}
              </p>
            )}
          </details>
        </>
      )}
      {busy && (
        <p role="status">Publishing and checking the exact cloud contents…</p>
      )}
      {message && <p role="status">{message}</p>}
      {error && <p role="alert">{error}</p>}
      <p>
        Local files are the source of truth. This publishes to Drive without
        importing cloud edits. Older local files stay in place unless you enable
        optional recycling.
      </p>
    </section>
  );
}
