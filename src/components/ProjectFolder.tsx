import { useEffect, useRef, useState } from "react";
import { AlertTriangle, FolderOpen } from "lucide-react";
import { registry } from "../services/registry";
import type { FolderPlan, Project } from "../types";
import { displayPath } from "../format";
import { Modal } from "./Modal";
import { OperationStatus } from "./OperationStatus";

export function ProjectFolder({
  project,
  busy,
  changed,
}: {
  project: Project;
  busy: boolean;
  changed: () => Promise<void>;
}) {
  const lost =
    project.folderState === "needs_reassignment" ||
    project.status === "unavailable";
  const [show, setShow] = useState(false);
  return (
    <>
      {lost ? (
        <section role="alert" className="panel folder-recovery">
          <h2>
            <AlertTriangle size={20} /> Folder lost — please reassign it
          </h2>
          <p>
            The registered folder is missing, inaccessible, or has been
            replaced. Find the project folder before working in this project
            again.
          </p>
          <p className="full-path">{displayPath(project.path)}</p>
          <p>
            You can close and reopen ContextDock. This reminder stays until you
            reassign the folder. Saved history and chats are retained.
          </p>
          <button
            className="primary"
            disabled={busy}
            onClick={() => setShow(true)}
          >
            <FolderOpen size={16} /> Reassign folder
          </button>
        </section>
      ) : (
        <button disabled={busy} onClick={() => setShow(true)}>
          <FolderOpen size={16} /> Reassign folder
        </button>
      )}
      {show && (
        <ReassignFolder
          key={project.id}
          project={project}
          changed={changed}
          close={() => setShow(false)}
        />
      )}
    </>
  );
}

function ReassignFolder({
  project,
  changed,
  close,
}: {
  project: Project;
  changed: () => Promise<void>;
  close: () => void;
}) {
  const [plan, setPlan] = useState<FolderPlan | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [page, setPage] = useState(0);
  const [saved, setSaved] = useState(false);
  const active = useRef(true);
  const working = useRef(false);
  const applied = useRef(false);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  async function pick() {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      const path = await registry.pickFolder();
      if (!path || !active.current) return;
      setPlan(null);
      setConfirmed(false);
      setPage(0);
      const next = await registry.previewFolder(project.id, path);
      if (active.current) setPlan(next);
    } catch (e) {
      if (active.current) {
        setPlan(null);
        setError(String(e));
      }
    } finally {
      working.current = false;
      if (active.current) setBusy(false);
    }
  }
  async function save() {
    if (working.current || !plan || !confirmed) return;
    working.current = true;
    setBusy(true);
    setError("");
    try {
      if (!applied.current) {
        await registry.reassignFolder(project.id, plan.path, plan.token);
        applied.current = true;
        if (active.current) setSaved(true);
      }
      const snapshot = await registry.snapshot();
      const next = snapshot.projects.find((p) => p.id === project.id);
      if (!next || next.path !== plan.path || next.folderState !== "ready") {
        throw new Error(
          "The folder assignment could not be verified. Check access to the selected folder and refresh verification.",
        );
      }
      await changed();
      if (active.current) close();
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      working.current = false;
      if (active.current) setBusy(false);
    }
  }
  return (
    <Modal
      title="Reassign project folder"
      close={() => {
        if (!working.current) close();
      }}
    >
      <p>
        Choose the project folder at its new location. Select the folder
        containing your documents, rather than its parent folder. You can also
        select the original folder after reconnecting a drive or restoring
        access.
      </p>
      <p>
        Your project, captured history, saved chats and cloud mappings stay
        together. Reassignment changes the saved location; it does not move your
        files.
      </p>
      <p className="full-path">Previous folder: {displayPath(project.path)}</p>
      <button disabled={busy || saved} onClick={() => void pick()}>
        <FolderOpen size={16} />{" "}
        {plan ? "Choose another folder" : "Choose folder"}
      </button>
      <OperationStatus busy={busy} message="Checking folder…" />
      {plan && (
        <section className="folder-review">
          <h3>Review selected folder</h3>
          <p className="full-path">{displayPath(plan.path)}</p>
          <p>
            {plan.found} known files found · {plan.unchanged} unchanged ·{" "}
            {plan.changed} changed · {plan.additional} additional documents
          </p>
          {plan.missing.length > 0 && (
            <div className="folder-missing" role="status">
              <strong>
                {plan.missing.length} files missing from the selected folder
              </strong>
              {plan.found === 0 && (
                <p>
                  No known files were found. Check that you selected the correct
                  project folder.
                </p>
              )}
              <ul>
                {plan.missing
                  .slice(page * 100, (page + 1) * 100)
                  .map((path) => (
                    <li key={path}>{path}</li>
                  ))}
              </ul>
              {plan.missing.length > 100 && (
                <div className="pagination">
                  <button
                    disabled={page === 0 || busy}
                    onClick={() => setPage((p) => p - 1)}
                  >
                    Previous missing files
                  </button>
                  <span>
                    Page {page + 1} of {Math.ceil(plan.missing.length / 100)}
                  </span>
                  <button
                    disabled={(page + 1) * 100 >= plan.missing.length || busy}
                    onClick={() => setPage((p) => p + 1)}
                  >
                    Next missing files
                  </button>
                </div>
              )}
              <p>
                Missing files keep their saved history. Restore them to this
                folder and refresh documents. Questions about missing selected
                files stay blocked.
              </p>
            </div>
          )}
          <label className="cloud-check">
            <input
              type="checkbox"
              disabled={busy || saved}
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />{" "}
            This is the folder for {project.name}. I have reviewed the file
            checks and any missing files.
          </label>
        </section>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {saved && (
        <p role="status">
          Folder assignment saved. Verifying the project before continuing.
        </p>
      )}
      <div className="actions">
        <button disabled={busy} onClick={close}>
          Cancel
        </button>
        <button
          className="primary"
          disabled={busy || !plan || !confirmed}
          onClick={() => void save()}
        >
          {saved ? "Refresh verification" : "Confirm reassignment"}
        </button>
      </div>
    </Modal>
  );
}
