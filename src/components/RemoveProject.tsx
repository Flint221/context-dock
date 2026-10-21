import { useEffect, useState } from "react";
import { cloud, type CloudView } from "../cloud";
import {
  cleanupPayload,
  hasCloudData,
  verifyCloudDeleted,
} from "../cloudVerification";
import { registry } from "../services/registry";
import { beginOperation } from "../operations";
import { Modal } from "./Modal";
import { OperationStatus } from "./OperationStatus";
import type { Snapshot } from "../types";

export function RemoveProject({
  project,
  close,
  removed,
}: {
  project: { id: string; name: string };
  close: () => void;
  removed: (snapshot: Snapshot) => void;
}) {
  const [view, setView] = useState<CloudView | null>(null);
  const [retain, setRetain] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    void cloud
      .view(project.id)
      .then((v) => {
        if (active) setView(v);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [project.id]);
  async function confirm() {
    if (!view) return;
    const release = beginOperation(`cloud:${project.id}`);
    if (!release) return;
    setBusy(true);
    setError("");
    try {
      if (!retain && hasCloudData(view)) {
        await cloud.action(project.id, "delete_cloud", cleanupPayload(view));
        const cleared = await cloud.view(project.id);
        verifyCloudDeleted(cleared);
        setView(cleared);
      }
      await registry.remove(project.id, retain);
      const snapshot = await registry.snapshot();
      if (snapshot.projects.some((p) => p.id === project.id))
        throw new Error(
          "Project removal could not be verified. Refresh and retry.",
        );
      close();
      removed(snapshot);
    } catch (e) {
      setError(String(e));
    } finally {
      release();
      setBusy(false);
    }
  }
  return (
    <Modal
      title="Remove project?"
      close={() => {
        if (!busy) close();
      }}
    >
      <p>
        Remove <strong>{project.name}</strong> from ContextDock? Your source
        folder, snapshots, version history and saved chats stay on this
        computer. Registering the same folder restores its local history with
        tracking off.
      </p>
      {!view && !error && <p role="status">Checking project cloud data…</p>}
      {view && hasCloudData(view) && (
        <fieldset disabled={busy}>
          <legend>What should happen to its cloud data?</legend>
          <label className="cloud-check">
            <input
              type="radio"
              name="cloud-retention"
              checked={!retain}
              onChange={() => setRetain(false)}
            />
            Delete the project vector store and its uploaded Files from OpenAI
          </label>
          <label className="cloud-check">
            <input
              type="radio"
              name="cloud-retention"
              checked={retain}
              onChange={() => setRetain(true)}
            />
            Keep cloud data in OpenAI (storage charges may continue)
          </label>
          <p>
            Cloud project: <strong>{view.store_name || project.name}</strong>.{" "}
            {retain
              ? "This explicitly retains remote data and stops local sync."
              : "Removal waits for verified cloud deletion. If cleanup fails, the project stays available for retry."}
          </p>
        </fieldset>
      )}
      {error && <p role="alert">{error}</p>}
      <OperationStatus
        busy={busy}
        message={
          retain
            ? "Stopping sync and removing project…"
            : "Deleting cloud data, verifying removal and removing project…"
        }
      />
      <div className="actions">
        <button disabled={busy} onClick={close}>
          Cancel
        </button>
        {!view && (
          <button
            disabled={busy}
            onClick={() => {
              setError("");
              void cloud
                .view(project.id)
                .then(setView)
                .catch((e) => setError(String(e)));
            }}
          >
            Retry cloud status
          </button>
        )}
        <button
          className="danger"
          disabled={busy || !view}
          onClick={() => void confirm()}
        >
          {busy
            ? "Removing…"
            : retain && view && hasCloudData(view)
              ? "Remove project and keep cloud data"
              : "Remove project"}
        </button>
      </div>
    </Modal>
  );
}
