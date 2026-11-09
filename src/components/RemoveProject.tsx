import { useState } from "react";
import { registry } from "../services/registry";
import { beginOperation } from "../operations";
import { Modal } from "./Modal";
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function confirm() {
    const release = beginOperation(`remove-project:${project.id}`);
    if (!release) return;
    setBusy(true);
    setError("");
    try {
      await registry.remove(project.id, true);
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
        Remove <strong>{project.name}</strong> from ContextDock and stop its
        monitoring and publication?
      </p>
      <p>
        Your local files and cloud copies stay in place. Existing archives and
        saved chats are also kept. Registering this folder again starts with
        monitoring and publication off.
      </p>
      <div className="toolbar-actions">
        <button
          className="danger"
          disabled={busy}
          onClick={() => void confirm()}
        >
          Remove project
        </button>
        <button disabled={busy} onClick={close}>
          Cancel
        </button>
      </div>
      {busy && <p role="status">Removing the project registration…</p>}
      {error && <p role="alert">{error}</p>}
    </Modal>
  );
}
