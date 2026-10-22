import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  Save,
} from "lucide-react";
import { registry } from "../services/registry";
import type { PathRule, PathSelectionView } from "../types";
import { useUnsavedChanges } from "../draftGuard";
import { beginOperation } from "../operations";
import { OperationStatus } from "./OperationStatus";
const key = (path: string) => path.replaceAll("\\", "/").toLowerCase();
const ruleSignature = (rules: PathRule[]) =>
  JSON.stringify(
    rules
      .map((r) => ({ path: key(r.path), kind: r.kind }))
      .sort(
        (a, b) => a.path.localeCompare(b.path) || a.kind.localeCompare(b.kind),
      ),
  );
const pathIncluded = (rules: PathRule[], path: string) =>
  !rules.some(
    (r) =>
      key(r.path) === key(path) ||
      (r.kind === "folder" &&
        (!r.path || key(path).startsWith(`${key(r.path)}/`))),
  );
interface Node {
  path: string;
  name: string;
  kind: "folder" | "file";
  children: Node[];
  status: string;
  isNew: boolean;
}
function nodes(view: PathSelectionView): Node {
  const root: Node = {
    path: "",
    name: "Project folder",
    kind: "folder",
    children: [],
    status: "available",
    isNew: false,
  };
  const map = new Map<string, Node>([["", root]]);
  const add = (path: string, kind: "folder" | "file", status: string, isNew = false): Node => {
    path = path.replaceAll("\\", "/");
    const id = key(path);
    const existing = map.get(id);
    if (existing) return existing;
    const split = path.lastIndexOf("/");
    const parent = add(
      split < 0 ? "" : path.slice(0, split),
      "folder",
      "available",
    );
    const node: Node = {
      path,
      name: path.slice(split + 1),
      kind,
      children: [],
      status,
      isNew,
    };
    map.set(id, node);
    parent.children.push(node);
    return node;
  };
  view.entries.forEach((e) => add(e.path, e.kind, e.status, e.new));
  view.exclusions.forEach((e) => add(e.path, e.kind, "missing"));
  for (const node of map.values())
    node.children.sort(
      (a, b) =>
        (a.kind === b.kind ? 0 : a.kind === "folder" ? -1 : 1) ||
        a.name.localeCompare(b.name),
    );
  return root;
}
function TreeRow({
  node,
  rules,
  toggle,
  disabled,
  ancestorExcluded = false,
}: {
  node: Node;
  rules: PathRule[];
  toggle: (node: Node, include: boolean) => void;
  disabled: boolean;
  ancestorExcluded?: boolean;
}) {
  const [open, setOpen] = useState(node.path === "");
  const [limit, setLimit] = useState(100);
  const included = pathIncluded(rules, node.path);
  const mixed =
    included &&
    node.kind === "folder" &&
    rules.some((r) =>
      node.path === ""
        ? !!r.path
        : key(r.path).startsWith(`${key(node.path)}/`),
    );
  const checkbox = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (checkbox.current) checkbox.current.indeterminate = mixed;
  }, [mixed]);
  return (
    <li className="folder-tree-node">
      <div className={`folder-tree-row ${included ? "" : "excluded"}`}>
        {node.kind === "folder" ? (
          <button
            className="tree-expander"
            aria-label={`${open ? "Collapse" : "Expand"} ${node.name}`}
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          </button>
        ) : (
          <span className="tree-expander" />
        )}
        <label>
          <input
            ref={checkbox}
            type="checkbox"
            aria-label={`Include ${node.path || "project folder"}`}
            aria-checked={mixed ? "mixed" : included}
            checked={included}
            disabled={disabled || ancestorExcluded}
            onChange={(e) => toggle(node, mixed || e.target.checked)}
          />
          {node.kind === "folder" ? (
            <Folder size={18} />
          ) : (
            <FileText size={17} />
          )}
          <span title={node.path}>{node.name}</span>
        </label>
        {!included && <span className="badge paused">Excluded</span>}
        {node.isNew && <span className="badge warning">New</span>}
        {node.status !== "available" && node.path && (
          <span className="badge missing">{node.status}</span>
        )}
      </div>
      {node.kind === "folder" && open && (
        <ul>
          {node.children.length ? (
            node.children
              .slice(0, limit)
              .map((child) => (
                <TreeRow
                  key={key(child.path)}
                  node={child}
                  rules={rules}
                  toggle={toggle}
                  disabled={disabled}
                  ancestorExcluded={ancestorExcluded || !included}
                />
              ))
          ) : (
            <li className="tree-empty">
              Empty folder · saved rules also cover future files
            </li>
          )}
          {node.children.length > limit && (
            <li>
              <button onClick={() => setLimit((n) => n + 100)}>
                Show 100 more in {node.name}
              </button>
            </li>
          )}
        </ul>
      )}
    </li>
  );
}
export function FolderTree({
  project,
  changed,
  unavailable = false,
}: {
  project: string;
  changed: () => Promise<void>;
  unavailable?: boolean;
}) {
  const [view, setView] = useState<PathSelectionView | null>(null);
  const [draft, setDraft] = useState<PathRule[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [reload, setReload] = useState(0);
  const mounted = useRef(false);
  const guard = useUnsavedChanges();
  const dirty =
    !!view && ruleSignature(draft) !== ruleSignature(view.exclusions);
  useEffect(() => {
    mounted.current = true;
    let current = true;
    void registry
      .paths(project)
      .then((v) => {
        if (current) {
          setView(v);
          setDraft(v.exclusions);
          setError("");
        }
      })
      .catch((e) => {
        if (current) setError(String(e));
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
      mounted.current = false;
    };
  }, [project, reload]);
  const discard = useCallback(() => {
    if (view) {
      setDraft(view.exclusions);
      setError("");
      setNotice("");
    }
  }, [view]);
  const save = useCallback(async () => {
    if (!view) return false;
    if (unavailable) {
      setError(
        "Reassign the missing project folder before saving. The draft is retained.",
      );
      return false;
    }
    const release = beginOperation(`cloud:${project}`);
    if (!release) {
      setError(
        "Another cloud or folder action is running. Wait for it to finish, then save again.",
      );
      return false;
    }
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const current = await registry.paths(project);
      if (ruleSignature(current.exclusions) !== ruleSignature(draft)) {
        if (current.revision !== view.revision)
          throw new Error(
            "Saved folder choices changed. Discard this draft and reload before editing again.",
          );
        let commandError: unknown;
        try {
          await registry.savePaths(project, view.revision, draft);
        } catch (e) {
          commandError = e;
        }
        const saved = await registry.paths(project);
        if (ruleSignature(saved.exclusions) !== ruleSignature(draft))
          throw (
            commandError ??
            new Error(
              "The backend did not confirm your folder selection. Your draft is retained.",
            )
          );
        setView(saved);
        setDraft(saved.exclusions);
      } else {
        setView(current);
        setDraft(current.exclusions);
      }
      setNotice(
        "Folder selection saved. Local history retained; cloud index changes run in the background. Excluded folders also cover future files.",
      );
      try {
        await changed();
      } catch (e) {
        setError(`Selection saved, but workspace refresh failed. ${String(e)}`);
      }
      return true;
    } catch (e) {
      if (mounted.current) setError(String(e));
      return false;
    } finally {
      release();
      if (mounted.current) setBusy(false);
    }
  }, [view, draft, project, changed, unavailable]);
  useLayoutEffect(
    () =>
      guard.register(`paths:${project}`, {
        dirty,
        saving: busy,
        save,
        discard,
      }),
    [guard, project, dirty, busy, save, discard],
  );
  const root = useMemo(() => (view ? nodes(view) : null), [view]);
  const toggle = (node: Node, include: boolean) => {
    setNotice("");
    setError("");
    setDraft((previous) => {
      const path = key(node.path);
      const remaining = previous.filter(
        (r) =>
          !(
            key(r.path) === path ||
            (node.kind === "folder" &&
              (path === "" || key(r.path).startsWith(`${path}/`)))
          ),
      );
      return include ? remaining : [...remaining, { path, kind: node.kind }];
    });
  };
  return (
    <section className="panel folder-selection" aria-label="Folder selection">
      <div className="section-heading">
        <div>
          <h2>
            <Folder size={18} /> Folder structure and selection
          </h2>
          <p>
            Checkboxes control tracking and this project's cloud index. Changes
            take effect only after Save changes.
          </p>
        </div>
        {dirty && <span className="badge warning">Unsaved changes</span>}
      </div>
      <p>
        Unchecking a folder excludes everything inside it, including future
        files. Local files, snapshots and saved history remain. Discovery still
        lists supported documents; uploaded cloud Files may remain available for
        reuse after their index entries are removed.
      </p>
      <p>
        New documents are excluded until selected and saved. Checked paths
        follow your Tracking and Cloud sync settings.
      </p>
      {error && <p role="alert">{error}</p>}
      {unavailable && (
        <p role="alert">
          Reassign the missing project folder before saving. Your draft remains
          here; you can discard it to leave without applying changes.
        </p>
      )}
      {!view && !error && <p role="status">Reading folder structure…</p>}
      {view?.warnings.map((warning, i) => (
        <p className="warning" key={i}>
          {warning}
        </p>
      ))}
      {!!view?.cloudPending && (
        <p className="warning">
          {view.cloudPending} cloud index removals are still pending. Check
          Cloud sync for failures or retries; local exclusion is already saved.
        </p>
      )}
      {root && (
        <ul className="folder-tree" aria-label="Project folder structure">
          <TreeRow
            node={root}
            rules={draft}
            toggle={toggle}
            disabled={busy || loading || unavailable || !guard.ready}
          />
        </ul>
      )}
      <OperationStatus
        busy={busy}
        message="Saving and verifying folder selection…"
        success={notice}
      />
      <div className="toolbar-actions">
        <button
          className="primary"
          disabled={!dirty || busy || loading || unavailable || !guard.ready}
          onClick={() => void save()}
        >
          <Save size={16} /> Save changes
        </button>
        <button disabled={!dirty || busy} onClick={discard}>
          Discard changes
        </button>
        <button
          disabled={dirty || busy || loading}
          onClick={() => {
            setLoading(true);
            setReload((r) => r + 1);
          }}
        >
          Reload folder tree
        </button>
      </div>
    </section>
  );
}
