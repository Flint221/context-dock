import { useState } from "react";
import { LatestDocuments } from "./LatestDocuments";
import { DrivePanel } from "./DrivePanel";
import {
  AlertTriangle,
  ArrowLeft,
  Copy,
  FolderOpen,
  Pencil,
  RefreshCw,
  Trash2,
} from "lucide-react";
import type { Project, Document, PathSelectionView } from "../types";
import { date, displayPath } from "../format";
import { ProjectFolder } from "./ProjectFolder";
import { FolderTree } from "./FolderTree";
import type { ProjectTab } from "./ProjectTabs";
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
  tab = "files",
}: {
  tab?: ProjectTab;
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
  const [selection, setSelection] = useState<PathSelectionView | null>(null);
  const folderLost =
    project.folderState === "needs_reassignment" ||
    project.status === "unavailable";
  const missing = documents.filter((d) => d.status === "missing");
  const inaccessible = documents.filter((d) => d.status === "unavailable");
  return (
    <>
      <div className="project-utilities">
        <button className="back" disabled={busy} onClick={back}>
          <ArrowLeft size={16} /> All projects
        </button>
        <ProjectFolder
          key={`folder:${project.id}`}
          project={project}
          busy={busy}
          changed={changed}
        />
      </div>
      <div
        hidden={tab !== "files"}
        role="tabpanel"
        id="project-panel-files"
        aria-labelledby="project-tab-files"
      >
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
              or fix access, then refresh documents.
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
        <details className="panel" open={!project.trackingEnabled}>
          <summary>Choose files and folders</summary>
          <FolderTree
            key={`paths:${project.id}`}
            project={project.id}
            changed={changed}
            unavailable={folderLost}
            onSelection={setSelection}
            refreshKey={`${project.path}:${project.lastScan}:${project.trackingRevision}`}
          />
        </details>
        {!folderLost && (
          <LatestDocuments
            key={`tracking:${project.id}`}
            project={project}
            changed={changed}
            selection={selection}
          />
        )}
      </div>
      {!folderLost && tab === "cloud" && (
        <div
          role="tabpanel"
          id="project-panel-cloud"
          aria-labelledby="project-tab-cloud"
        >
          <DrivePanel key={`drive:${project.id}`} project={project.id} />
        </div>
      )}
      {folderLost && tab !== "files" && (
        <p className="banner warning">
          Reassign the project folder above to access Drive publishing. Your
          files are kept.
        </p>
      )}
    </>
  );
}
