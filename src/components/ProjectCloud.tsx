import { useCallback, useEffect, useRef, useState } from "react";
import { cloud, bytes, type CloudView, type Draft, type Plan } from "../cloud";
import { CloudPanel } from "./CloudPanel";
import { ProjectChat } from "./ProjectChat";
import { Modal } from "./Modal";
import { beginOperation } from "../operations";
import {
  verifyCloudAction,
  verifyCloudDraft,
  verifyCloudDeleted,
  cleanupPayload,
  hasCloudData,
} from "../cloudVerification";
import { OperationStatus } from "./OperationStatus";
import { ProgressBar } from "./ProgressBar";

export function ProjectCloud({ project }: { project: string }) {
  const [view, setView] = useState<CloudView | null>(null);
  const [stateKnown, setStateKnown] = useState(false);
  const [busy, setBusy] = useState(false);
  const [advancedBusy, setAdvancedBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [stop, setStop] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [setup, setSetup] = useState<{ draft: Draft; plan: Plan } | null>(null);
  const alive = useRef(true);
  const readSequence = useRef(0);
  const load = useCallback(async () => {
    const sequence = ++readSequence.current;
    try {
      const next = await cloud.view(project);
      if (alive.current && sequence === readSequence.current) {
        setView(next);
        setStateKnown(true);
      }
      return next;
    } catch (e) {
      if (alive.current && sequence === readSequence.current)
        setStateKnown(false);
      throw e;
    }
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
  async function run(work: () => Promise<unknown>) {
    const release = beginOperation(`cloud:${project}`);
    if (!release) return;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await work();
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      if (alive.current) {
        setBusy(false);
      }
      release();
    }
  }
  const enabled = !!view?.enabled && !view.paused;
  const cleanupNeeded =
    !!view && ["cleanup_pending", "cleanup_failed"].includes(view.status);
  const retained = !!view && !enabled && hasCloudData(view);
  const selected = view?.families.filter((f) => f.selected && f.eligible) ?? [];
  const indexedTotal = view
    ? Math.max(view.indexed + view.pending + view.failed, selected.length)
    : 0;
  const awaitingSelection = view?.families.filter((f) => f.pathExcluded).length ?? 0;
  const awaitingCapture = view?.families.filter((f) => !f.pathExcluded && f.versionCount === 0).length ?? 0;
  const awaitingUpload = selected.filter((f) => f.currentNumber !== f.cloudNumber).length;
  const needsAttention = view?.families.filter((f) => f.selected && !f.eligible && f.versionCount > 0).length ?? 0;
  const pathsExcluded = !!view?.families.some(
    (f) => f.selected && f.pathExcluded,
  );
  const ready =
    enabled &&
    view?.localReady !== false &&
    view?.connectionStatus === "connected" &&
    !!view.store_id &&
    view.failed === 0 &&
    view.pending === 0 &&
    awaitingCapture === 0 &&
    needsAttention === 0 &&
    selected.length > 0 &&
    selected.every((f) => f.currentNumber === f.cloudNumber);
  const reason = !view
    ? "Loading cloud status…"
    : view.localError
      ? view.localError
      : view.connectionStatus !== "connected"
        ? "Save and test your OpenAI key in Settings to connect."
        : view.error
          ? view.error
          : pathsExcluded && selected.length === 0
            ? "Selected documents are excluded by the folder tree. Include a path and Save changes to make its selected cloud documents eligible again."
            : selected.length === 0
              ? "Select files in the folder tree and enable version tracking."
              : "Your documents are syncing. Questions unlock when the current versions are ready.";
  function enable() {
    if (!view) return;
    void run(async () => {
      const eligible = view.families.filter((f) => f.selected && f.eligible);
      if (!eligible.length)
        throw new Error(
          "Select files in the folder tree, enable document tracking, and capture an eligible document before turning on Cloud sync.",
        );
      const draft: Draft = {
        enabled: true,
        automatic: true,
        mode: view.mode,
        documents: eligible.map((f) => f.id),
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
              {!stateKnown
                ? busy
                  ? "Checking cloud status…"
                  : "Cloud status unavailable · Refresh status"
                : cleanupNeeded
                  ? "Cloud deletion incomplete · Retry cleanup"
                  : retained
                    ? "Paused · Cloud data retained"
                    : enabled
                      ? ready
                        ? "On · Documents ready for questions"
                        : view.failed > 0
                          ? "On · Some files need attention"
                          : pathsExcluded && selected.length === 0
                            ? "On · Selected paths excluded"
                            : "On · Preparing your documents"
                      : "Off · Local version history"}
            </p>
          </div>
          <label className="cloud-check cloud-switch">
            <input
              type="checkbox"
              role="switch"
              aria-label="Cloud sync"
              checked={!!view?.enabled}
              disabled={
                busy ||
                advancedBusy ||
                !!setup ||
                stop ||
                !view ||
                !stateKnown ||
                cleanupNeeded
              }
              aria-busy={busy}
              onChange={(e) => {
                if (e.target.checked) enable();
                else setStop(true);
              }}
            />
            {!stateKnown
              ? busy
                ? "Checking…"
                : "Unknown"
              : enabled
                ? "On"
                : cleanupNeeded
                  ? "Cleanup needed"
                  : retained
                    ? "Paused"
                    : "Off"}
          </label>
        </div>
        <p>
          {enabled
            ? "Your selected documents sync to OpenAI. Ask project uses their current indexed versions."
            : retained
              ? "Cloud data is still retained. Delete it below to fully turn off Cloud sync; local history and chats stay available."
              : "Your documents and version history stay on this computer. Turn on Cloud sync to ask AI questions."}
        </p>
        {view && view.connectionStatus !== "connected" && (
          <p>First, open Settings → OpenAI connection → Save and test key.</p>
        )}
        {error && !setup && !stop && <p role="alert">{error}</p>}
        {view?.error && <p role="alert">{view.error}</p>}
        {!!view?.selectionRemovalPending && (
          <div className="banner warning">
            <span>
              {view.selectionRemovalPending} excluded index entries await
              verified removal. Local exclusion is already saved.
            </span>
            <button
              disabled={busy || advancedBusy}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "retry_selection_removals");
                  await load();
                })
              }
            >
              Retry index removals
            </button>
          </div>
        )}
        <OperationStatus
          busy={busy || advancedBusy}
          message="Saving and checking cloud status…"
          success={notice}
        />
        {view && stateKnown && (
          <p className="tracking-note">
            Folder coverage: {awaitingSelection} excluded or awaiting selection · {awaitingCapture} awaiting capture · {awaitingUpload} awaiting cloud sync · {needsAttention} need local review.
            Upload failures are reported separately below.
          </p>
        )}
        {view &&
          stateKnown &&
          !cleanupNeeded &&
          !!view.enabled &&
          !!view.started &&
          indexedTotal > 0 && (
            <div className="cloud-index-progress">
              <ProgressBar
                label="Indexed versions"
                value={(100 * view.indexed) / indexedTotal}
                detail={`${view.indexed} of ${indexedTotal} indexed`}
              />
              <small className={view.failed ? "error" : undefined}>
                {view.paused ? "Uploads paused · " : ""}
                {view.pending} pending · {view.failed} failed or uncertain
                {!ready &&
                  " · Questions unlock when selected current versions are ready."}
              </small>
            </div>
          )}
        <div className="toolbar-actions">
          <button
            disabled={busy || advancedBusy}
            onClick={() => void run(load)}
          >
            Refresh status
          </button>
          {retained && (
            <button
              className="danger"
              disabled={busy || advancedBusy || !stateKnown}
              onClick={() => setStop(true)}
            >
              Delete cloud data
            </button>
          )}
          {enabled && (
            <button
              className="primary"
              aria-busy={busy || advancedBusy}
              disabled={busy || advancedBusy}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "sync");
                  verifyCloudAction(await load(), "sync");
                  if (alive.current)
                    setNotice(
                      "Sync requested and confirmed. Document indexing may still be in progress.",
                    );
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
          {advanced && (
            <CloudPanel
              key={project}
              project={project}
              onBusyChange={(saving) => {
                setAdvancedBusy(saving);
                if (saving) setNotice("");
              }}
              onChanged={load}
            />
          )}
        </details>
      </section>
      <ProjectChat
        key={project}
        project={project}
        enabled={enabled}
        ready={ready && stateKnown && !busy && !advancedBusy}
        reason={
          stateKnown
            ? reason
            : "Cloud status could not be verified. Refresh status before asking a question."
        }
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
            . Change document selection in the folder tree or the version policy
            in Advanced cloud settings.
          </p>
          <p>
            OpenAI receives these files. Sending a question also sends that
            question, recent messages in its chat and retrieved document
            passages. Storage and AI requests may incur charges. Files remain
            online until explicitly deleted. Chats are saved locally.
          </p>
          {error && <p role="alert">{error}</p>}
          <OperationStatus
            busy={busy}
            message="Enabling Cloud sync and checking the saved settings…"
          />
          <div className="toolbar-actions">
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  setStateKnown(false);
                  await cloud.action(project, "apply", {
                    draft: setup.draft,
                    token: setup.plan.token,
                    consent: true,
                  });
                  verifyCloudDraft(await load(), setup.draft);
                  setStateKnown(false);
                  await cloud.action(project, "start");
                  verifyCloudAction(await load(), "start");
                  if (alive.current) {
                    setSetup(null);
                    setNotice(
                      "Cloud sync enabled and confirmed. Waiting for document indexing to finish.",
                    );
                  }
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
            Permanently delete <strong>{view?.store_name}</strong>, its vector
            store, and all uploaded Files belonging to this project from OpenAI.
            New uploads and AI questions stop first. Local documents, snapshots,
            version history and chats stay available. Turning sync on again
            creates a fresh bucket and re-uploads your selected versions. This
            waits for remote deletion verification; a failed cleanup remains
            visible for retry.
          </p>
          {error && <p role="alert">{error}</p>}
          <OperationStatus
            busy={busy}
            message="Deleting cloud data and checking the saved state…"
          />
          <div className="toolbar-actions">
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  setStateKnown(false);
                  if (!view) return;
                  try {
                    await cloud.action(
                      project,
                      "delete_cloud",
                      cleanupPayload(view),
                    );
                    verifyCloudDeleted(await load());
                  } catch (e) {
                    await load().catch(() => {});
                    throw e;
                  }
                  if (alive.current) {
                    setStop(false);
                    setNotice(
                      "Cloud sync is off — confirmed. Project vector store and uploaded Files deleted and verified. Local history and saved chats remain available.",
                    );
                  }
                })
              }
            >
              {busy ? "Turning off…" : "Turn off Cloud sync"}
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
