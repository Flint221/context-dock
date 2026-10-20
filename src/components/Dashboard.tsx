import { AlertTriangle, ArrowUpRight, FileText, Folder } from "lucide-react";
import type { Snapshot, Project } from "../types";
import { date } from "../format";
import { Projects } from "./Projects";
export function Dashboard({
  data,
  busy,
  showProject,
  viewAll,
}: {
  data: Snapshot;
  busy: boolean;
  showProject: (p: Project) => Promise<void>;
  viewAll: () => void;
}) {
  const recentProjects = [...data.projects]
    .sort((a, b) =>
      (b.latestModified || "").localeCompare(a.latestModified || ""),
    )
    .slice(0, 5);
  const recentDocs = [...data.documents]
    .sort((a, b) =>
      (b.modifiedAt || b.lastObserved).localeCompare(
        a.modifiedAt || a.lastObserved,
      ),
    )
    .slice(0, 6);
  const unavailable = data.projects.filter((p) => p.status !== "available");

  return (
    <>
      <div className="stats">
        <section className="panel">
          <Folder size={21} />
          <span>Registered projects</span>
          <strong>{data.projects.length}</strong>
        </section>
        <section className="panel">
          <FileText size={21} />
          <span>Discovered documents</span>
          <strong>{data.documents.length}</strong>
          <small>
            {data.documents.filter((d) => d.status === "available").length}{" "}
            available
          </small>
        </section>
        <section className="panel">
          <AlertTriangle size={21} />
          <span>Folders needing attention</span>
          <strong>{unavailable.length}</strong>
        </section>
      </div>
      {unavailable.length > 0 && (
        <section className="panel attention">
          <h2>Folders needing attention</h2>
          {unavailable.map((p) => (
            <button
              key={p.id}
              disabled={busy}
              onClick={() => void showProject(p)}
            >
              <AlertTriangle size={16} />
              {p.name}
              <span className={`badge ${p.status}`}>
                {p.folderState === "needs_reassignment"
                  ? "Folder lost · Reassign"
                  : p.status}
              </span>
            </button>
          ))}
        </section>
      )}
      <div className="section-heading">
        <div>
          <h2>Recent projects</h2>
          <p>Ordered by the latest observed file modification.</p>
        </div>
        <button onClick={viewAll}>
          View all <ArrowUpRight size={16} />
        </button>
      </div>
      {
        <Projects
          projects={recentProjects}
          busy={busy}
          showProject={showProject}
        />
      }
      <section className="panel recent">
        <div className="section-heading">
          <div>
            <h2>Recent documents</h2>
            <p>Recently modified or discovered across your workspace.</p>
          </div>
        </div>
        {recentDocs.length ? (
          recentDocs.map((d) => (
            <button
              key={d.id}
              disabled={busy}
              onClick={() => {
                const p = data.projects.find((p) => p.id === d.projectId);
                if (p) void showProject(p);
              }}
            >
              <FileText size={18} />
              <span>
                <strong>{d.name}</strong>
                <small>
                  {data.projects.find((p) => p.id === d.projectId)?.name} /{" "}
                  {d.relativePath}
                </small>
              </span>
              <span className={`badge ${d.status}`}>{d.status}</span>
              <time>{date(d.modifiedAt || d.lastObserved)}</time>
            </button>
          ))
        ) : (
          <p className="padded">
            No supported documents discovered yet. Open a project to refresh its
            folder.
          </p>
        )}
      </section>
    </>
  );
}
