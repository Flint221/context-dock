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
export function ProjectDetails({
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
  return (
    <>
      <button className="back" disabled={busy} onClick={back}>
        <ArrowLeft size={16} /> All projects
      </button>
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
          <button className="danger" disabled={busy} onClick={remove}>
            <Trash2 size={16} />
            Remove
          </button>
        </div>
      </section>
      {project.scanError && (
        <div className="banner warning">
          <AlertTriangle size={18} />
          <span>{project.scanError}</span>
        </div>
      )}
      <div className="refresh-row">
        <span className={`badge ${project.status}`}>{project.status}</span>
        <button disabled={busy} onClick={refresh}>
          <RefreshCw size={16} />
          Refresh documents
        </button>
      </div>
      <Documents key={project.id + project.lastScan} documents={documents} />
    </>
  );
}
