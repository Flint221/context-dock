import { useState, type FormEvent } from "react";
import { FolderOpen } from "lucide-react";
import { registry } from "../services/registry";
import { Modal } from "./Modal";
import { OperationStatus } from "./OperationStatus";
export function AddProject({
  close,
  save,
}: {
  close: () => void;
  save: (name: string, path: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(e: FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await save(name.trim(), path);
      close();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <Modal
      title="Add a project"
      close={() => {
        if (!saving) close();
      }}
    >
      <p>
        Register an existing folder. ContextDock discovers document metadata
        without changing your files.
      </p>
      <form onSubmit={submit}>
        <label>
          Project name
          <input
            autoFocus
            data-autofocus
            required
            maxLength={120}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Cattle Tool"
            disabled={saving}
          />
        </label>
        <label>
          Project folder
          <div className="folder-picker">
            <output>{path || "No folder selected"}</output>
            <button
              type="button"
              disabled={saving}
              onClick={async () => {
                try {
                  const p = await registry.pickFolder();
                  if (p) setPath(p);
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              <FolderOpen size={16} /> Browse
            </button>
          </div>
        </label>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <OperationStatus busy={saving} message="Discovering documents…" />
        <div className="actions">
          <button type="button" disabled={saving} onClick={close}>
            Cancel
          </button>
          <button
            className="primary"
            aria-busy={saving}
            disabled={saving || !name.trim() || !path}
          >
            {saving ? "Discovering documents…" : "Add project"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
