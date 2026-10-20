import { useCallback, useEffect, useRef, useState } from "react";
import { cloud, bytes, type CloudView, type Draft, type Plan } from "../cloud";
import { CloudPanel } from "./CloudPanel";
import { ProjectChat } from "./ProjectChat";
import { Modal } from "./Modal";

export function ProjectCloud({ project }: { project: string }) {
  const [view, setView] = useState<CloudView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [stop, setStop] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [setup, setSetup] = useState<{ draft: Draft; plan: Plan } | null>(null);
  const alive = useRef(true);
  const load = useCallback(async () => {
    const next = await cloud.view(project);
    if (alive.current) setView(next);
  }, [project]);
  useEffect(() => {
    alive.current = true;
    const refresh = () =>
      void load().catch((e) => {
        if (alive.current) setError(String(e));
      });
    refresh();
    let unsubscribe: (() => void) | undefined;
    void cloud
      .subscribe(refresh)
      .then((fn) => {
        if (alive.current) unsubscribe = fn;
        else fn();
      })
      .catch(() => {});
    return () => {
      alive.current = false;
      unsubscribe?.();
    };
  }, [load]);
  async function run(work: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await work();
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      if (alive.current) {
        await load().catch(() => {});
        setBusy(false);
      }
    }
  }
  const enabled = !!view?.enabled && !view.paused;
  const selected = view?.families.filter((f) => f.selected && f.eligible) ?? [];
  const ready =
    enabled &&
    view?.connectionStatus === "connected" &&
    !!view.store_id &&
    view.failed === 0 &&
    view.pending === 0 &&
    selected.length > 0 &&
    selected.every((f) => f.currentNumber === f.cloudNumber);
  const reason = !view
    ? "Loading cloud status…"
    : view.connectionStatus !== "connected"
      ? "Save and test your OpenAI key in Settings to connect."
      : view.error
        ? view.error
        : selected.length === 0
          ? "Enable version tracking and select documents for cloud sync."
          : "Your documents are syncing. Questions unlock when the current versions are ready.";
  function enable() {
    if (!view) return;
    void run(async () => {
      const eligible = view.families.filter((f) => f.eligible);
      if (!eligible.length)
        throw new Error(
          "Enable document tracking below and capture a document first, then turn on Cloud sync.",
        );
      const saved = eligible.filter((f) => f.selected);
      const draft: Draft = {
        enabled: true,
        automatic: true,
        mode: view.mode,
        documents: (saved.length ? saved : eligible).map((f) => f.id),
        softLimit: view.soft_limit,
      };
      const plan = await cloud.action<Plan>(project, "preview", { draft });
      if (alive.current) setSetup({ draft, plan });
    });
  }
  return (
    <>
      <section className="panel simple-cloud" aria-label="Cloud sync">
        <div className="cloud-heading">
          <div>
            <h2>Cloud sync</h2>
            <p>
              {enabled
                ? ready
                  ? "On · Documents ready for questions"
                  : "On · Preparing your documents"
                : "Off · Local version history"}
            </p>
          </div>
          <label className="cloud-check cloud-switch">
            <input
              type="checkbox"
              role="switch"
              aria-label="Cloud sync"
              checked={enabled}
              disabled={busy || !view}
              onChange={(e) => {
                if (e.target.checked) enable();
                else setStop(true);
              }}
            />
            {enabled ? "On" : "Off"}
          </label>
        </div>
        <p>
          {enabled
            ? "Your selected documents sync to OpenAI. Ask project uses their current indexed versions."
            : "Your documents and version history stay on this computer. Turn on Cloud sync to ask AI questions."}
        </p>
        {view && view.connectionStatus !== "connected" && (
          <p>First, open Settings → OpenAI connection → Save and test key.</p>
        )}
        {error && !setup && !stop && <p role="alert">{error}</p>}
        {enabled && view?.error && <p role="alert">{view.error}</p>}
        <div className="toolbar-actions">
          <button disabled={busy} onClick={() => void run(load)}>
            Refresh status
          </button>
          {enabled && (
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "sync");
                })
              }
            >
              Sync now
            </button>
          )}
        </div>
        <details
          onToggle={(e) => {
            const open = e.currentTarget.open;
            setAdvanced(open);
            if (!open) void load().catch(() => {});
          }}
        >
          <summary>Advanced cloud settings</summary>
          {advanced && <CloudPanel key={project} project={project} />}
        </details>
      </section>
      <ProjectChat
        key={project}
        project={project}
        enabled={enabled}
        ready={ready}
        reason={reason}
      />
      {setup && (
        <Modal
          title="Enable Cloud sync and AI?"
          close={() => {
            if (!busy) {
              setSetup(null);
              setError("");
            }
          }}
        >
          <p>
            Sync these documents to OpenAI and enable questions for this
            project. New captured versions will sync automatically while
            ContextDock is open.
          </p>
          <ul>
            {view?.families
              .filter((f) => setup.draft.documents.includes(f.id))
              .map((f) => (
                <li key={f.id}>{f.name}</li>
              ))}
          </ul>
          <p>
            {setup.plan.files} snapshots · {bytes(setup.plan.bytes)} ·{" "}
            {setup.draft.mode === "current"
              ? "Current versions"
              : "Captured history"}
            . Change the document selection or version policy in Advanced cloud
            settings.
          </p>
          <p>
            OpenAI receives these files. Sending a question also sends that
            question, recent messages in its chat and retrieved document
            passages. Storage and AI requests may incur charges. Files remain
            online until explicitly deleted. Chats are saved locally.
          </p>
          {error && <p role="alert">{error}</p>}
          <div className="toolbar-actions">
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "apply", {
                    draft: setup.draft,
                    token: setup.plan.token,
                    consent: true,
                  });
                  if (alive.current) setSetup(null);
                  await cloud.action(project, "start");
                })
              }
            >
              {busy ? "Enabling…" : "Enable Cloud sync and AI"}
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setSetup(null);
                setError("");
              }}
            >
              Cancel
            </button>
          </div>
        </Modal>
      )}
      {stop && (
        <Modal
          title="Turn off Cloud sync?"
          close={() => {
            if (!busy) {
              setStop(false);
              setError("");
            }
          }}
        >
          <p>
            Stop new uploads and AI requests for this project. Local version
            history and saved chats stay available. Cloud files remain online;
            an already-sent request may finish.
          </p>
          {error && <p role="alert">{error}</p>}
          <div className="toolbar-actions">
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "disconnect");
                  if (alive.current) setStop(false);
                })
              }
            >
              Turn off Cloud sync
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setStop(false);
                setError("");
              }}
            >
              Cancel
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
