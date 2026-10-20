import { displayPath } from "./format";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  Activity,
  Check,
  Folder,
  FolderOpen,
  LayoutDashboard,
  Plus,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { registry } from "./services/registry";
import type { Project, Snapshot } from "./types";

import { AddProject } from "./components/AddProject";
import { Projects } from "./components/Projects";
import { Dashboard } from "./components/Dashboard";
import { SettingsView } from "./components/Settings";
import { ProjectDetails } from "./components/ProjectDetails";
import { RemoveProject } from "./components/RemoveProject";
import { Modal } from "./components/Modal";
import { CurrentProcesses } from "./components/CurrentProcesses";
import { OperationStatus } from "./components/OperationStatus";
import { version } from "../package.json";
type Page = "dashboard" | "projects" | "processes" | "settings";
export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [data, setData] = useState<Snapshot>({
    projects: [],
    documents: [],
    theme: "system",
  });
  const [selected, setSelected] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [editing, setEditing] = useState<"rename" | "remove" | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const active = useRef(false);
  const readSequence = useRef(0);
  const load = useCallback(async () => {
    const sequence = ++readSequence.current;
    const snapshot = await registry.snapshot();
    if (active.current && sequence === readSequence.current) {
      setData(snapshot);
      setLoaded(true);
    }
  }, []);
  useEffect(() => {
    active.current = true;
    const refresh = () =>
      void load().catch((e) => {
        if (active.current) setError(String(e));
      });
    refresh();
    let unlisten: (() => void) | undefined;
    registry
      .subscribe(refresh)
      .then((stop) => {
        if (active.current) unlisten = stop;
        else stop();
      })
      .catch((e) => {
        if (active.current) setError(String(e));
      });
    return () => {
      active.current = false;
      unlisten?.();
    };
  }, [load]);
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        data.theme === "system"
          ? media.matches
            ? "dark"
            : "light"
          : data.theme;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [data.theme]);
  async function run(action: () => Promise<unknown>, success?: string) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
      await load();
      if (success) setNotice(success);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  const project = data.projects.find((p) => p.id === selected);
  async function showProject(p: Project) {
    setPage("projects");
    setSelected(p.id);
    await run(() => registry.refresh(p.id));
  }

  return (
    <div className="app-shell">
      <aside>
        <div className="brand">
          <span className="brand-icon">
            <Folder size={23} />
          </span>
          <div>
            ContextDock<small>PROJECT WORKSPACE</small>
          </div>
        </div>
        <div className="nav-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          {(
            [
              { id: "dashboard", label: "Dashboard", Icon: LayoutDashboard },
              { id: "projects", label: "Projects", Icon: Folder },
              { id: "processes", label: "Current processes", Icon: Activity },
              { id: "settings", label: "Settings", Icon: Settings },
            ] as const
          ).map(({ id, label, Icon }) => (
            <button
              key={id}
              aria-current={page === id ? "page" : undefined}
              className={page === id ? "active" : ""}
              onClick={() => {
                setPage(id);
                setSelected(null);
              }}
            >
              <Icon size={18} />
              {label}
              {id === "projects" && (
                <span className="nav-count">{data.projects.length}</span>
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <ShieldCheck size={17} />
          <span>
            Local by design<small>Local history, optional cloud AI.</small>
          </span>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            Workspace <span>/</span>{" "}
            {page === "dashboard"
              ? "Dashboard"
              : page === "settings"
                ? "Settings"
                : page === "processes"
                  ? "Current processes"
                  : "Projects"}
            {project && page === "projects" && (
              <>
                {" "}
                <span>/</span> {project.name}
              </>
            )}
          </div>
          <span className="local-label">
            <span /> Offline ready
          </span>
        </header>
        <div className="content">
          {error && (
            <div role="alert" className="banner error">
              <AlertTriangle size={18} />
              <span>{error}</span>
              <button onClick={() => void run(load)}>Retry</button>
            </div>
          )}
          {notice && (
            <div role="status" className="banner">
              <Check size={16} />
              {notice}
            </div>
          )}
          <OperationStatus busy={busy} message="Reading local metadata…" />
          <div
            className="page-scene"
            key={`${page}:${page === "projects" ? (selected ?? "list") : ""}`}
          >
            <div className="page-heading">
              <div>
                <div className="eyebrow">
                  {page === "processes"
                    ? "LOCAL ACTIVITY, AT A GLANCE"
                    : page === "settings"
                      ? "MAKE YOURSELF AT HOME"
                      : "YOUR DOCUMENTATION, CONNECTED"}
                </div>
                <h1>
                  {page === "dashboard"
                    ? "Workspace overview"
                    : page === "settings"
                      ? "Settings"
                      : page === "processes"
                        ? "Current processes"
                        : project?.name || "Projects"}
                </h1>
                <p>
                  {page === "dashboard"
                    ? "A clear view of your projects and the documents behind them."
                    : page === "settings"
                      ? "Make ContextDock feel at home."
                      : page === "processes"
                        ? "See what’s running locally. Preview it, identify it, and stop it when you’re done."
                        : project
                          ? "Track document history and ask about your synced files."
                          : "Existing folders. Independent documents. One place to find them."}
                </p>
              </div>
              {(page === "dashboard" || page === "projects") && (
                <button
                  className="primary"
                  disabled={busy || !loaded}
                  onClick={() => setAdding(true)}
                >
                  <Plus size={17} /> Add project
                </button>
              )}
            </div>
            {page === "processes" ? (
              <CurrentProcesses />
            ) : !loaded ? (
              <section className="panel empty">
                <h2>
                  {error
                    ? "Unable to load your workspace"
                    : "Opening your local workspace…"}
                </h2>
                <p>
                  {error
                    ? "Use Retry above to reconnect to the application."
                    : "Loading saved project registrations."}
                </p>
              </section>
            ) : page === "settings" ? (
              <SettingsView
                theme={data.theme}
                busy={busy}
                changeTheme={(theme) => void run(() => registry.theme(theme))}
              />
            ) : page === "projects" && project ? (
              <ProjectDetails
                project={project}
                changed={load}
                documents={data.documents.filter(
                  (d) => d.projectId === project.id,
                )}
                busy={busy}
                back={() => setSelected(null)}
                copy={() =>
                  void run(
                    () =>
                      navigator.clipboard.writeText(displayPath(project.path)),
                    "Folder path copied.",
                  )
                }
                open={() => void run(() => registry.open(project.id))}
                rename={() => {
                  setName(project.name);
                  setEditing("rename");
                }}
                remove={() => setEditing("remove")}
                refresh={() =>
                  void run(
                    () => registry.refresh(project.id),
                    "Document metadata refreshed.",
                  )
                }
              />
            ) : data.projects.length === 0 ? (
              <section className="panel empty welcome">
                <span className="welcome-icon">
                  <FolderOpen size={36} />
                </span>
                <h2>A home for your project documentation</h2>
                <p>
                  Register an existing folder with a name you recognize.
                  <br />
                  ContextDock finds its Markdown and Word documents, while your
                  files stay exactly where they are.
                </p>
                <button className="primary" onClick={() => setAdding(true)}>
                  <Plus size={17} />
                  Add your first project
                </button>
                <small>
                  Read-only discovery · Local storage · No account needed
                </small>
              </section>
            ) : page === "projects" ? (
              <Projects
                projects={data.projects}
                busy={busy}
                showProject={showProject}
              />
            ) : (
              <Dashboard
                data={data}
                busy={busy}
                showProject={showProject}
                viewAll={() => {
                  setPage("projects");
                  setSelected(null);
                }}
              />
            )}
          </div>
        </div>
        <footer>
          <span>ContextDock {version}</span>
          <span>Local snapshots · Source archival only when enabled</span>
        </footer>
      </main>
      {adding && (
        <AddProject
          close={() => setAdding(false)}
          save={async (n, p) => {
            setBusy(true);
            try {
              const id = await registry.add(n, p);
              await load();
              setSelected(id);
              setPage("projects");
            } finally {
              setBusy(false);
            }
          }}
        />
      )}
      {editing === "remove" && project && (
        <RemoveProject
          project={project}
          close={() => setEditing(null)}
          removed={async () => {
            await load();
            setSelected(null);
            setEditing(null);
            setNotice(
              "Project removed. Local files and captured history retained.",
            );
          }}
        />
      )}
      {editing === "rename" && project && (
        <Modal
          title="Rename project"
          close={() => {
            if (!busy) setEditing(null);
          }}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                await registry.rename(project.id, name);
                setEditing(null);
              });
            }}
          >
            <label>
              Project name
              <input
                autoFocus
                required
                maxLength={120}
                value={name}
                disabled={busy}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            {error && (
              <p role="alert" className="error">
                {error}
              </p>
            )}
            <div className="actions">
              <button
                type="button"
                disabled={busy}
                onClick={() => setEditing(null)}
              >
                Cancel
              </button>
              <button disabled={busy || !name.trim()} className="primary">
                {busy ? "Saving…" : "Save name"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
