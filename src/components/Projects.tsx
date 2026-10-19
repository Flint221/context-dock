import { ArrowUpRight, Folder } from "lucide-react";
import type { Project } from "../types";
import { date } from "../format";
export function Projects({
  projects,
  busy,
  showProject,
}: {
  projects: Project[];
  busy: boolean;
  showProject: (p: Project) => Promise<void>;
}) {
  return (
    <div className="project-grid">
      {projects.map((p) => (
        <button
          key={p.id}
          className="project-card"
          disabled={busy}
          onClick={() => void showProject(p)}
        >
          <div className="card-top">
            <span className="folder-icon">
              <Folder size={21} />
            </span>
            <ArrowUpRight size={17} />
          </div>
          <h3>{p.name}</h3>
          <p className="path" title={p.path}>
            {p.path.replace(/^\\\\\?\\/, "")}
          </p>
          <div className="card-bottom">
            <span>{p.documentCount} documents</span>
            <span className={`badge ${p.status}`}>{p.status}</span>
          </div>
          <small>Latest file: {date(p.latestModified)}</small>
        </button>
      ))}
    </div>
  );
}
