import { TrackingPanel } from "./TrackingPanel";
import { ProjectCloud } from "./ProjectCloud";
import {
  AlertTriangle,
  ArrowLeft,
  Copy,
  FolderOpen,
  Pencil,
  RefreshCw,
  Trash2,
} from "lucide-react";
import type { Project, Document } from "../types";
import { date, displayPath } from "../format";
import { Documents } from "./Documents";
import { ProjectFolder } from "./ProjectFolder";
export function ProjectDetails({
  changed,
  project,
  documents,
  busy,
  back,
  copy,
  open,
  rename,
  remove,
  refresh,
}: {
  changed: () => Promise<void>;
  project: Project;
  documents: Document[];
  busy: boolean;
  back: () => void;
  copy: () => void;
  open: () => void;
  rename: () => void;
  remove: () => void;
  refresh: () => void;
}) {
  const folderLost =
    project.folderState === "needs_reassignment" ||
    project.status === "unavailable";
  const missing = documents.filter((d) => d.status === "missing");
  const inaccessible = documents.filter((d) => d.status === "unavailable");
  return (
    <>
      <button className="back" disabled={busy} onClick={back}>
        <ArrowLeft size={16} /> All projects
      </button>
      <ProjectFolder
        key={`folder:${project.id}`}
        project={project}
        busy={busy}
        changed={changed}
      />
      <details className="project-options">
        <summary>Project folder and options</summary>
        <section className="panel project-toolbar">
          <div>
            <h2>Registered folder</h2>
            <p className="full-path">{displayPath(project.path)}</p>
            <small>Last scan: {date(project.lastScan)}</small>
          </div>
          <div className="toolbar-actions">
            <button disabled={busy} onClick={copy}>
              <Copy size={16} />
              Copy path
            </button>
            <button disabled={busy} onClick={open}>
              <FolderOpen size={16} />
              Explorer
            </button>
            <button disabled={busy} onClick={rename}>
              <Pencil size={16} />
              Rename
            </button>
          </div>
        </section>
      </details>
      {project.scanError && !folderLost && (
        <div role="alert" className="banner error">
          <AlertTriangle size={18} />
          <span>{project.scanError}</span>
        </div>
      )}
      {!folderLost && (missing.length > 0 || inaccessible.length > 0) && (
        <div role="status" className="banner warning">
          <AlertTriangle size={18} />
          <span>
            {missing.length} files missing · {inaccessible.length} files
            inaccessible. Check the document statuses below, restore the files
            or fix access, then refresh documents. Saved history is retained.
          </span>
        </div>
      )}
      <div className="refresh-row">
        <span className={`badge ${project.status}`}>{project.status}</span>
        <div className="toolbar-actions">
          <button className="danger" disabled={busy} onClick={remove}>
            <Trash2 size={16} /> Remove project
          </button>
          <button
            className="primary"
            disabled={busy}
            aria-busy={busy}
            onClick={refresh}
          >
            <RefreshCw size={16} />
            Refresh documents
          </button>
        </div>
      </div>
      {!folderLost && (
        <ProjectCloud
          key={`cloud:${project.id}:${project.path}`}
          project={project.id}
        />
      )}
      {!folderLost && (
        <TrackingPanel
          key={`tracking:${project.id}`}
          project={project}
          changed={changed}
        />
      )}
      <Documents key={`documents:${project.id}`} documents={documents} />
    </>
  );
}
