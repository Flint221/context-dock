import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
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
import { Modal } from "./components/Modal";
import { version } from "../package.json";
type Page = "dashboard" | "projects" | "settings";
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
  const load = useCallback(async () => {
    setData(await registry.snapshot());
    setLoaded(true);
  }, []);
  useEffect(() => {
    let active = true;
    registry
      .snapshot()
      .then((snapshot) => {
        if (active) {
          setData(snapshot);
          setLoaded(true);
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
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
            Local by design<small>Your files stay on your computer.</small>
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
          {busy && (
            <div role="status" className="progress">
              Reading local metadata…
            </div>
          )}
          <div className="page-heading">
            <div>
              <div className="eyebrow">YOUR DOCUMENTATION, CONNECTED</div>
              <h1>
                {page === "dashboard"
                  ? "Workspace overview"
                  : page === "settings"
                    ? "Settings"
                    : project?.name || "Projects"}
              </h1>
              <p>
                {page === "dashboard"
                  ? "A clear view of your projects and the documents behind them."
                  : page === "settings"
                    ? "Make ContextDock feel at home."
                    : project
                      ? "Documents discovered in your registered folder."
                      : "Existing folders. Independent documents. One place to find them."}
              </p>
            </div>
            {page !== "settings" && (
              <button
                className="primary"
                disabled={busy || !loaded}
                onClick={() => setAdding(true)}
              >
                <Plus size={17} /> Add project
              </button>
            )}
          </div>
          {!loaded ? (
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
              documents={data.documents.filter(
                (d) => d.projectId === project.id,
              )}
              busy={busy}
              back={() => setSelected(null)}
              copy={() =>
                void run(
                  () =>
                    navigator.clipboard.writeText(
                      project.path.replace(/^\\\\\?\\/, ""),
                    ),
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
        <footer>
          <span>ContextDock {version}</span>
          <span>
            Metadata discovery only · Your original files stay untouched
          </span>
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
      {editing && project && (
        <Modal
          title={editing === "rename" ? "Rename project" : "Remove project?"}
          close={() => {
            if (!busy) setEditing(null);
          }}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                if (editing === "rename")
                  await registry.rename(project.id, name);
                else {
                  await registry.remove(project.id);
                  setSelected(null);
                }
                setEditing(null);
              });
            }}
          >
            {editing === "rename" ? (
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
            ) : (
              <p>
                Remove <strong>{project.name}</strong> and its saved metadata
                from ContextDock? Your folder and all its files will remain on
                your computer.
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
              <button
                disabled={busy || (editing === "rename" && !name.trim())}
                className={editing === "remove" ? "danger" : "primary"}
              >
                {busy
                  ? "Saving…"
                  : editing === "rename"
                    ? "Save name"
                    : "Remove registration"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
