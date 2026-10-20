import { CloudLink } from "./CloudLink";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  cloud,
  bytes,
  type CloudView,
  type Draft,
  type Plan,
  type Hit,
  type Job,
} from "../cloud";
import { Modal } from "./Modal";
import { date } from "../format";
import { beginOperation } from "../operations";
import { verifyCloudAction, verifyCloudDraft } from "../cloudVerification";
import { OperationStatus } from "./OperationStatus";
const initial: Draft = {
  enabled: true,
  automatic: false,
  mode: "current",
  documents: [],
  softLimit: 50 * 1024 * 1024,
};
function savedDraft(view: CloudView): Draft {
  return {
    enabled: !!view.enabled || view.store_state === "none",
    automatic: !!view.auto_sync,
    mode: view.mode,
    documents: view.families.filter((f) => f.selected).map((f) => f.id),
    softLimit: view.soft_limit,
  };
}
export function CloudPanel({
  project,
  onBusyChange,
  onChanged,
}: {
  project: string;
  onBusyChange?: (busy: boolean) => void;
  onChanged?: () => Promise<unknown>;
}) {
  const [view, setView] = useState<CloudView | null>(null);
  const [draft, setDraft] = useState<Draft>(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [consent, setConsent] = useState(false);
  const [afterReview, setAfterReview] = useState<"start" | "sync" | null>(null);
  const [stopping, setStopping] = useState<"project" | "automatic" | null>(
    null,
  );
  const [transition, setTransition] = useState<"pause" | "resume" | null>(null);
  const [index, setIndex] = useState(false);
  const [hits, setHits] = useState<Hit[] | null>(null);
  const [query, setQuery] = useState("");
  const [destruction, setDestruction] = useState<{
    action: string;
    job?: Job;
  } | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const deletionText =
    destruction?.action === "delete_index"
      ? (view?.store_name ?? "")
      : `DELETE ${destruction?.job?.file_id ?? ""}`;
  const [attestation, setAttestation] = useState(false);
  const [recovery, setRecovery] = useState<Job | true | null>(null);
  const alive = useRef(true);
  const readSequence = useRef(0);
  const dirty = useRef(false);
  const load = useCallback(async () => {
    const sequence = ++readSequence.current;
    const next = await cloud.view(project);
    if (alive.current && sequence === readSequence.current) {
      setView(next);
      if (!dirty.current) setDraft(savedDraft(next));
    }
    return next;
  }, [project]);
  useEffect(() => {
    alive.current = true;
    let unlisten: (() => void) | undefined;
    const refresh = () => {
      void load().catch((e) => {
        if (alive.current) setError(String(e));
      });
    };
    refresh();
    void cloud
      .subscribe(refresh)
      .then((fn) => {
        if (alive.current) unlisten = fn;
        else fn();
      })
      .catch(() => {});
    return () => {
      alive.current = false;
      unlisten?.();
    };
  }, [load]);
  async function run(action: () => Promise<unknown>) {
    const release = beginOperation(`cloud:${project}`);
    if (!release) return;
    setBusy(true);
    onBusyChange?.(true);
    setError("");
    setNotice("");
    try {
      await action();
      await load();
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        setNotice("");
      }
      await load().catch(() => {});
    } finally {
      await onChanged?.().catch((e) => {
        if (alive.current) {
          setError(`Could not refresh confirmed cloud status: ${String(e)}`);
          setNotice("");
        }
      });
      release();
      onBusyChange?.(false);
      if (alive.current) setBusy(false);
    }
  }
  function edit(next: Partial<Draft>, prompt = true) {
    setNotice("");
    setError("");
    if (next.enabled === false && view?.enabled) {
      setStopping("project");
      return;
    }
    if (next.automatic === false && view?.auto_sync && view.enabled) {
      setStopping("automatic");
      return;
    }
    const updated = { ...draft, ...next };
    dirty.current = true;
    setDraft(updated);
    setPlan(null);
    if (
      prompt &&
      view &&
      (view.enabled || view.started) &&
      updated.enabled &&
      connectionReady
    ) {
      review(
        view.started &&
          (!view.paused || next.enabled === true || next.automatic === true)
          ? "sync"
          : null,
        updated,
      );
    }
  }
  function cancelChanges() {
    if (busy) return;
    dirty.current = false;
    if (view) setDraft(savedDraft(view));
    setPlan(null);
    setAfterReview(null);
    setStopping(null);
    setTransition(null);
    setError("");
  }
  function stopSync(kind: "project" | "automatic") {
    setError("");
    setStopping(kind);
  }
  async function perform(name: string) {
    await cloud.action(project, name);
    verifyCloudAction(await load(), name);
  }
  const action = (name: string) =>
    void run(async () => {
      await perform(name);
      setNotice(
        ["start", "sync", "resume"].includes(name)
          ? "Sync request confirmed. Indexing may still be in progress."
          : "Cloud action confirmed.",
      );
    });
  function manage(action: string, job?: Job) {
    setConfirmation("");
    setAttestation(false);
    setDestruction({ action, job });
  }
  const connectionReady = view?.connectionStatus === "connected";
  const selectionMissing =
    draft.enabled && !view?.started && draft.documents.length === 0;
  const hasChanges =
    !!view &&
    (draft.enabled !== (!!view.enabled || view.store_state === "none") ||
      draft.automatic !== !!view.auto_sync ||
      draft.mode !== view.mode ||
      draft.softLimit !== view.soft_limit ||
      view.families.some(
        (f) => !!f.selected !== draft.documents.includes(f.id),
      ));
  function review(
    nextAction: "start" | "sync" | null = null,
    selection = draft,
  ) {
    void run(async () => {
      const next = await cloud.action<Plan>(project, "preview", {
        draft: selection,
      });
      if (!alive.current) return;
      setAfterReview(nextAction);
      setConsent(false);
      setPlan(next);
    });
  }
  return (
    <section className="panel cloud-panel" aria-label="Cloud indexing">
      <div className="cloud-heading">
        <div>
          <h2>Cloud indexing</h2>
          <p>Optional OpenAI vector store · {view?.status ?? "Local only"}</p>
        </div>
        <button disabled={busy} onClick={() => void run(load)}>
          Refresh cloud status
        </button>
      </div>
      {error && !plan && !stopping && !index && !destruction && !recovery && (
        <p role="alert">{error}</p>
      )}
      <OperationStatus
        busy={busy}
        message="Applying and checking cloud settings… Local tracking remains available."
        success={notice}
      />
      {view?.error && <p role="status">{view.error}</p>}
      {view && !view.enabled && !view.started && (
        <div className="cloud-setup">
          <p>
            <strong>Finish cloud setup before initial upload.</strong> Checking
            Enable prepares a draft; the project stays local-only until
            configuration is successfully saved.
          </p>
          <p>
            1. Save and test your OpenAI key in Settings. 2. Select documents
            below. 3. Review and confirm cloud changes. 4. Click Start Initial
            Sync.
          </p>
        </div>
      )}
      {view && !connectionReady && (
        <div className="cloud-setup" role="status">
          <p>
            OpenAI connection: <strong>{view.connectionStatus}</strong>. A saved
            key must pass a connection test before cloud indexing can be
            enabled. If no key is saved, open Settings → OpenAI connection →
            Save and test key.
          </p>
          <button
            disabled={busy}
            onClick={() => void run(() => cloud.action(null, "test"))}
          >
            Test saved OpenAI key
          </button>
        </div>
      )}
      <p>
        Only explicitly selected, tracked document families can be uploaded.
        Discovery alone never uploads files. Pausing stops the next request; an
        in-flight request may finish. Local tracking continues independently.
      </p>
      {view && (
        <>
          <div className="cloud-facts">
            <span>{view.indexed} indexed</span>
            <span>{view.pending} pending</span>
            <span>{view.failed} failed / uncertain</span>
            <span>Raw indexed snapshots: {bytes(view.rawBytes)}</span>
            <span>OpenAI index: {bytes(view.usage_bytes)}</span>
            <span>{view.retainedFiles} uploaded Files retained</span>
          </div>
          <p>
            Index: <code>{view.store_id ?? "Not created"}</code> · Created{" "}
            {date(view.created_at)} · Last verified/synced{" "}
            {date(view.last_sync)}
          </p>
          <fieldset disabled={busy}>
            <legend>Project cloud selection</legend>
            {hasChanges && (
              <p role="status">
                Unsaved cloud changes. Finish editing the upload limit to
                review, or use Review cloud changes below.
              </p>
            )}
            <label className="cloud-check">
              <input
                type="checkbox"
                checked={draft.enabled}
                onChange={(e) => edit({ enabled: e.target.checked })}
              />
              Enable cloud indexing for this project
            </label>
            <label className="cloud-check">
              <input
                type="checkbox"
                checked={draft.automatic}
                onChange={(e) => edit({ automatic: e.target.checked })}
              />
              Automatically sync newly captured versions while ContextDock is
              open
            </label>
            <div className="cloud-form">
              <label>
                Version policy
                <select
                  value={draft.mode}
                  onChange={(e) =>
                    edit({ mode: e.target.value as Draft["mode"] })
                  }
                >
                  <option value="current">
                    Current version only (recommended)
                  </option>
                  <option value="history">Full captured history</option>
                </select>
              </label>
              <label>
                Soft upload limit (MiB)
                <input
                  type="number"
                  min={1}
                  max={512}
                  value={draft.softLimit / (1024 * 1024)}
                  onChange={(e) =>
                    edit(
                      { softLimit: Number(e.target.value) * 1024 * 1024 },
                      false,
                    )
                  }
                  onBlur={() => {
                    if (
                      hasChanges &&
                      view.enabled &&
                      connectionReady &&
                      !plan &&
                      !busy
                    )
                      review();
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                  }}
                />
              </label>
            </div>
            <p>
              The soft limit blocks individual uploads; it does not enforce a
              billing cap. Application safety limits: 16 MiB raw, 4 MiB
              extracted text/expanded DOCX; OpenAI also enforces type and token
              limits.
            </p>
            <div className="cloud-family-list">
              <p>
                <strong>
                  Documents to index · {draft.documents.length} selected
                </strong>
              </p>
              {selectionMissing && (
                <p>Select at least one eligible document below to continue.</p>
              )}
              <button
                type="button"
                disabled={!view.families.some((f) => f.eligible)}
                onClick={() =>
                  edit({
                    documents: view.families
                      .filter((f) => f.eligible)
                      .map((f) => f.id),
                  })
                }
              >
                Select all eligible documents
              </button>
              {view.families.map((f) => (
                <label className="cloud-family" key={f.id}>
                  <input
                    type="checkbox"
                    disabled={!f.eligible && !f.selected}
                    checked={draft.documents.includes(f.id)}
                    onChange={(e) =>
                      edit({
                        documents: e.target.checked
                          ? [...draft.documents, f.id]
                          : draft.documents.filter((id) => id !== f.id),
                      })
                    }
                  />
                  <span>
                    <strong>{f.name}</strong> · local v{f.currentNumber ?? "–"}{" "}
                    · cloud v{f.cloudNumber ?? "–"}
                    <small>
                      {f.versionCount} captured · {bytes(f.rawBytes)} total raw
                      ·{" "}
                      {f.eligible
                        ? "Eligible"
                        : "Resolve local tracking / ambiguity before upload"}
                    </small>
                  </span>
                </label>
              ))}
            </div>
            <button
              disabled={(draft.enabled && !connectionReady) || selectionMissing}
              onClick={() => review()}
            >
              Review cloud changes
            </button>
          </fieldset>
          <div className="cloud-actions">
            {view.store_state === "creating" && !view.store_id && (
              <button
                disabled={busy}
                onClick={() => {
                  setConfirmation("");
                  setAttestation(false);
                  setRecovery(true);
                }}
              >
                Retry confirmed absent index creation
              </button>
            )}
            <button
              className="primary"
              disabled={busy || !view.enabled || !!view.started}
              onClick={() => (hasChanges ? review("start") : action("start"))}
            >
              Start Initial Sync
            </button>
            <button
              disabled={busy || !view.enabled || !view.started}
              onClick={() => (hasChanges ? review("sync") : action("sync"))}
            >
              Sync Now
            </button>
            <button
              disabled={busy || !view.enabled}
              onClick={() => setTransition(view.paused ? "resume" : "pause")}
            >
              {view.paused ? "Resume cloud sync" : "Pause cloud sync"}
            </button>
            <button
              disabled={busy || !view.enabled}
              onClick={() => action("reconcile")}
            >
              Retry / Reconcile / Verify key
            </button>
            <button
              disabled={busy || !view.enabled}
              onClick={() => stopSync("project")}
            >
              Stop cloud syncing
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setHits(null);
                setIndex(true);
              }}
            >
              View Cloud Index / Test Search
            </button>
            {view.store_id && (
              <button
                className="danger"
                disabled={busy}
                onClick={() => manage("delete_index")}
              >
                Delete project cloud index
              </button>
            )}
            {view.store_state === "deleted" && (
              <button
                disabled={busy}
                onClick={() =>
                  void run(() =>
                    cloud.action(project, "new_index", { confirm: true }),
                  )
                }
              >
                Prepare a new index (review consent again)
              </button>
            )}
          </div>
          <p>
            Disconnecting, removing a project or uninstalling ContextDock does
            not delete remote resources. Automatic replacement detaches old
            associations only after replacements finish. Uploaded Files are
            retained until you explicitly delete them separately.
          </p>
          <details>
            <summary>Recent cloud activity ({view.activity.length})</summary>
            {view.activity.map((a, i) => (
              <p key={`${a.created_at}:${i}`}>
                <small>{date(a.created_at)}</small> · {a.message}
              </p>
            ))}
          </details>
        </>
      )}
      {stopping && (
        <Modal
          title={
            stopping === "project" ? "Stop syncing?" : "Stop automatic syncing?"
          }
          close={cancelChanges}
        >
          {error && (
            <p role="alert">Could not confirm syncing stopped. {error}</p>
          )}
          <OperationStatus
            busy={busy}
            message="Stopping sync and checking the saved state…"
          />
          <p>
            {stopping === "project"
              ? "Stop cloud syncing for this project."
              : "Turn off automatic cloud sync and pause queued work. You can still use Sync Now for a manual sync."}{" "}
            No further requests will start after this is saved. An in-flight
            request may finish. Local tracking continues. Existing cloud files
            and the index are kept.
          </p>
          <div className="cloud-actions">
            <button
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await perform(
                    stopping === "project" ? "disconnect" : "stop_automatic",
                  );
                  dirty.current = false;
                  setStopping(null);
                  setNotice(
                    stopping === "project"
                      ? "Cloud syncing stopped."
                      : "Automatic syncing stopped. Use Sync Now when you want to sync manually.",
                  );
                })
              }
            >
              {stopping === "project"
                ? "Stop syncing"
                : "Stop automatic syncing"}
            </button>
            <button disabled={busy} onClick={cancelChanges}>
              Cancel
            </button>
          </div>
        </Modal>
      )}
      {plan && (
        <Modal title="Confirm OpenAI cloud indexing" close={cancelChanges}>
          <OperationStatus
            busy={busy}
            message="Saving and checking cloud configuration…"
          />
          {error && (
            <p role="alert">
              <strong>Cloud configuration could not be confirmed.</strong>{" "}
              {error}
            </p>
          )}
          <p>
            Store name: <strong>{plan.storeName}</strong>. OpenAI will receive
            exact immutable Markdown/DOCX snapshots and opaque ContextDock
            ownership/version identifiers. Local absolute paths are not sent.
            Project names and document contents may contain sensitive
            information.
          </p>
          <p>
            {plan.files} selected snapshots · {bytes(plan.bytes)} raw ·{" "}
            {plan.additions} additions/reindex operations · {plan.removals}{" "}
            association removals.
          </p>
          <p>
            OpenAI retains Files and vector-store data until deleted. Detaching
            an older version or deleting its index does not delete its
            underlying File. API data is not used for model training by default.
            Storage/processing charges may apply, and free storage allowances
            are shared across your account.
          </p>
          <p>
            <CloudLink href="https://developers.openai.com/api/docs/guides/your-data">
              OpenAI data controls
            </CloudLink>{" "}
            ·{" "}
            <CloudLink href="https://developers.openai.com/api/docs/pricing">
              Current pricing
            </CloudLink>
          </p>
          <label className="cloud-check">
            <input
              type="checkbox"
              checked={consent}
              disabled={busy}
              onChange={(e) => setConsent(e.target.checked)}
            />
            I consent to these uploads, removals, retention and potential API
            charges.
          </label>
          <div className="cloud-actions">
            <button
              disabled={busy || !consent}
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "apply", {
                    draft,
                    token: plan.token,
                    consent: true,
                  });
                  verifyCloudDraft(await load(), draft);
                  dirty.current = false;
                  setPlan(null);
                  if (afterReview && draft.enabled) {
                    setNotice("Cloud configuration saved. Requesting sync…");
                    await perform(afterReview);
                    setNotice("Cloud configuration saved. Sync requested.");
                    return;
                  }
                  setNotice(
                    draft.enabled
                      ? view?.started
                        ? "Cloud configuration saved. Changes are queued; resume cloud sync if it is paused."
                        : "Cloud configuration saved. Click Start Initial Sync to begin uploading your selected documents."
                      : "Cloud configuration saved.",
                  );
                })
              }
            >
              {afterReview && draft.enabled
                ? afterReview === "start"
                  ? "Confirm and start initial sync"
                  : "Confirm and sync"
                : "Confirm cloud configuration"}
            </button>
            <button disabled={busy} onClick={cancelChanges}>
              Cancel
            </button>
          </div>
          <p>
            New projects still require Start Initial Sync after confirmation.
            Changes to an already-started index queue only the changes shown
            above.
          </p>
        </Modal>
      )}
      {transition && (
        <Modal
          title={
            transition === "pause" ? "Pause cloud sync?" : "Resume cloud sync?"
          }
          close={cancelChanges}
        >
          <p>
            {transition === "pause"
              ? "Pause new uploads and AI requests. Local history and saved chats remain available; an already-sent request may finish."
              : "Resume cloud syncing for the saved document selection and allow AI questions when the current versions are indexed. API charges may apply."}
          </p>
          {error && <p role="alert">{error}</p>}
          <OperationStatus
            busy={busy}
            message="Saving and checking cloud status…"
          />
          <div className="cloud-actions">
            <button
              disabled={busy}
              aria-busy={busy}
              onClick={() =>
                void run(async () => {
                  await perform(transition);
                  setNotice(
                    transition === "pause"
                      ? "Cloud sync paused — confirmed."
                      : "Cloud sync resumed — confirmed. Indexing may still be in progress.",
                  );
                  setTransition(null);
                })
              }
            >
              {busy
                ? "Saving…"
                : transition === "pause"
                  ? "Pause cloud sync"
                  : "Resume cloud sync"}
            </button>
            <button disabled={busy} onClick={cancelChanges}>
              Cancel
            </button>
          </div>
        </Modal>
      )}
      {index && view && (
        <Modal
          title="Cloud index and retrieval test"
          close={() => {
            if (!busy) setIndex(false);
          }}
        >
          {error && !destruction && !recovery && <p role="alert">{error}</p>}
          <p>
            <code>{view.store_id ?? "No cloud index"}</code> · Direct
            vector-store retrieval only; no generated answer or AI analysis.
          </p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () =>
                setHits(
                  await cloud.action<Hit[]>(project, "search", { query }),
                ),
              );
            }}
          >
            <label>
              Test Search
              <input
                value={query}
                maxLength={2000}
                onChange={(e) => setQuery(e.target.value)}
                disabled={busy}
              />
            </label>
            <button
              disabled={busy || !query.trim() || !view.enabled || !!view.paused}
            >
              Search cloud index
            </button>
          </form>
          {hits && (
            <div aria-live="polite">
              {hits.length === 0 ? (
                <p>No results returned.</p>
              ) : (
                hits.map((hit, i) => (
                  <article className="cloud-result" key={`${hit.fileId}:${i}`}>
                    <strong>
                      {hit.familyName} · v{hit.number}
                    </strong>
                    <p>
                      Score {hit.score ?? "not supplied"} ·{" "}
                      <code>{hit.fileId}</code> · snapshot {hit.versionId}
                    </p>
                    <pre>{hit.excerpt}</pre>
                  </article>
                ))
              )}
            </div>
          )}
          <h3>Known cloud versions</h3>
          <p>
            Removed associations are excluded from future automatic sync. Files
            are separate retained resources. Failed/uncertain rows require
            reconciliation.
          </p>
          {view.jobs.map((job) => (
            <article className="cloud-result" key={job.id}>
              <strong>
                {job.familyName} · v{job.number} · {job.state}
              </strong>
              <p>
                Raw {bytes(job.rawBytes)} · indexed {bytes(job.usage_bytes)} ·{" "}
                <code>{job.file_id ?? "No confirmed File ID"}</code>
              </p>
              {job.error && <p>{job.error}</p>}
              <div className="cloud-actions">
                {["failed", "uncertain", "uploaded"].includes(job.state) && (
                  <button
                    disabled={busy}
                    onClick={() => {
                      setConfirmation("");
                      setAttestation(false);
                      setRecovery(job);
                    }}
                  >
                    Retry confirmed missing upload
                  </button>
                )}
                <button
                  disabled={
                    busy ||
                    !job.file_id ||
                    ["removed", "purged"].includes(job.state)
                  }
                  onClick={() => manage("remove", job)}
                >
                  Remove association
                </button>
                <button
                  className="danger"
                  disabled={busy || job.state !== "removed" || !job.file_id}
                  onClick={() => manage("purge_file", job)}
                >
                  Delete uploaded File
                </button>
              </div>
            </article>
          ))}
        </Modal>
      )}
      {recovery && (
        <Modal
          title="Confirm recovery retry"
          close={() => {
            if (!busy) setRecovery(null);
          }}
        >
          {error && <p role="alert">{error}</p>}
          <p>
            Use Retry / Reconcile first. Only continue if the previous request
            has ended and you verified that no matching index or File exists in
            the correct OpenAI account. ContextDock checks the accessible remote
            listing again before retrying. A delayed or inaccessible copy can
            still create uncertainty.
          </p>
          <label>
            Type RETRY ABSENT OPERATION
            <input
              disabled={busy}
              value={confirmation}
              onChange={(e) => setConfirmation(e.target.value)}
            />
          </label>
          <label className="cloud-check">
            <input
              type="checkbox"
              checked={attestation}
              onChange={(e) => setAttestation(e.target.checked)}
            />
            I checked the correct OpenAI account, the previous request has
            ended, and no remote copy exists.
          </label>
          <div className="cloud-actions">
            <button
              disabled={
                busy ||
                confirmation !== "RETRY ABSENT OPERATION" ||
                !attestation
              }
              onClick={() =>
                void run(async () => {
                  await cloud.action(project, "retry_absent", {
                    version: recovery === true ? null : recovery.version_id,
                    confirmation,
                    noOtherReferences: attestation,
                  });
                  setRecovery(null);
                })
              }
            >
              Confirm recovery retry
            </button>
            <button disabled={busy} onClick={() => setRecovery(null)}>
              Cancel
            </button>
          </div>
        </Modal>
      )}
      {destruction && (
        <Modal
          title="Confirm remote deletion"
          close={() => {
            if (!busy) setDestruction(null);
          }}
        >
          {error && <p role="alert">{error}</p>}
          <p>
            Local documents, snapshots and version history remain unchanged.
          </p>
          {destruction.action === "remove" ? (
            <p>
              Remove this version's vector-store association and exclude it from
              future synchronization? The uploaded File remains in OpenAI.
            </p>
          ) : (
            <>
              <p>
                {destruction.action === "delete_index"
                  ? "Delete this entire project vector store and its associations? Underlying uploaded Files remain for separate cleanup."
                  : "Permanently delete this uploaded File from OpenAI? This may affect other resources that use it."}
              </p>
              <label>
                Type {deletionText} to confirm
                <input
                  value={confirmation}
                  onChange={(e) => setConfirmation(e.target.value)}
                  disabled={busy}
                />
              </label>
            </>
          )}
          {destruction.action === "purge_file" && (
            <label className="cloud-check">
              <input
                type="checkbox"
                checked={attestation}
                onChange={(e) => setAttestation(e.target.checked)}
              />
              I verified that no other tools, threads, batches or resources use
              this File. ContextDock also checks every accessible vector store;
              it cannot discover references outside this key's access.
            </label>
          )}
          <div className="cloud-actions">
            <button
              className="danger"
              disabled={
                busy ||
                (destruction.action !== "remove" &&
                  (!deletionText || confirmation !== deletionText)) ||
                (destruction.action === "purge_file" && !attestation)
              }
              onClick={() =>
                void run(async () => {
                  await cloud.action(
                    project,
                    destruction.action === "remove"
                      ? "remove_now"
                      : destruction.action,
                    {
                      version: destruction.job?.version_id,
                      confirm: true,
                      confirmation,
                      storeId: view?.store_id,
                      noOtherReferences: attestation,
                    },
                  );
                  setDestruction(null);
                })
              }
            >
              Confirm remote deletion
            </button>
            <button disabled={busy} onClick={() => setDestruction(null)}>
              Cancel
            </button>
          </div>
        </Modal>
      )}
    </section>
  );
}
