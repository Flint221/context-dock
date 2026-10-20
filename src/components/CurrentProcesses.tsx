import { useCallback, useEffect, useRef, useState } from "react";
import {
  Activity,
  Copy,
  ExternalLink,
  RefreshCw,
  Square,
  ShieldCheck,
} from "lucide-react";
import {
  processes,
  processKey,
  isLocalEndpoint,
  endpointLabel,
} from "../services/processes";
import type {
  LocalProcess,
  ProcessInventory,
  StartupItem,
  StopResult,
} from "../services/processes";
import { Modal } from "./Modal";
import { OperationStatus } from "./OperationStatus";

const PAGE_SIZE = 25;
export function CurrentProcesses() {
  const [inventory, setInventory] = useState<ProcessInventory | null>(null);
  const [readError, setReadError] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [search, setSearch] = useState("");
  const [mode, setMode] = useState("listeners");
  const [protocol, setProtocol] = useState("all");
  const [scope, setScope] = useState("all");
  const [secure, setSecure] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [review, setReview] = useState<LocalProcess[] | null>(null);
  const [results, setResults] = useState<StopResult[]>([]);
  const [startup, setStartup] = useState<StartupItem[] | null>(null);
  const [page, setPage] = useState(0);
  const active = useRef(false);
  const reading = useRef(false);
  const acting = useRef(false);
  const refresh = useCallback(async () => {
    if (reading.current) return;
    reading.current = true;
    setLoading(true);
    try {
      const next = await processes.inventory();
      if (!active.current) return;
      setInventory(next);
      setReadError("");
      const eligible = new Set(
        next.processes.filter((p) => p.canStop).map(processKey),
      );
      setSelected(
        (previous) => new Set([...previous].filter((key) => eligible.has(key))),
      );
    } catch (e) {
      if (active.current) setReadError(String(e));
    } finally {
      reading.current = false;
      if (active.current) setLoading(false);
    }
  }, []);
  useEffect(() => {
    active.current = true;
    const initialRead = window.setTimeout(() => void refresh(), 0);
    return () => {
      active.current = false;
      window.clearTimeout(initialRead);
    };
  }, [refresh]);
  useEffect(() => {
    if (!autoRefresh || review) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible" && !acting.current)
        void refresh();
    }, 5000);
    return () => window.clearInterval(timer);
  }, [autoRefresh, review, refresh]);

  async function action(task: () => Promise<unknown>, success?: string) {
    if (acting.current) return;
    acting.current = true;
    setWorking(true);
    setError("");
    setNotice("");
    try {
      await task();
      if (active.current && success) setNotice(success);
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      acting.current = false;
      if (active.current) setWorking(false);
    }
  }
  async function stopReviewed() {
    if (!review || acting.current) return;
    const targets = review;
    await action(async () => {
      const stopped = await processes.stop(targets);
      if (!active.current) return;
      setResults(stopped);
      setReview(null);
      setSelected(new Set());
      await refresh();
    });
  }
  const query = search.trim().toLowerCase();
  const visible = (inventory?.processes ?? []).flatMap((p) => {
    const endpoints = p.endpoints.filter(
      (e) =>
        (protocol === "all" || e.protocol === protocol) &&
        (scope === "all" || isLocalEndpoint(e)),
    );
    if (
      !endpoints.length &&
      (mode === "listeners" ||
        p.endpoints.length > 0 ||
        protocol !== "all" ||
        scope !== "all")
    )
      return [];
    if (
      query &&
      !`${p.name} ${p.path} ${p.pid} ${p.parentPid} ${endpoints.map(endpointLabel).join(" ")}`
        .toLowerCase()
        .includes(query)
    )
      return [];
    return [{ process: p, endpoints }];
  });
  const eligible = visible.map((v) => v.process).filter((p) => p.canStop);
  const chosen = eligible.filter((p) => selected.has(processKey(p)));
  const pageCount = Math.max(1, Math.ceil(visible.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  const rows = visible.slice(
    currentPage * PAGE_SIZE,
    (currentPage + 1) * PAGE_SIZE,
  );
  const disabled = working || !!readError || !inventory;
  const endpointCount =
    inventory?.processes.reduce((n, p) => n + p.endpoints.length, 0) ?? 0;
  const filterChanged = () => {
    setPage(0);
    setSelected(new Set());
  };
  return (
    <div className="process-view">
      <section className="panel process-overview" aria-label="Process overview">
        <div className="process-stats">
          <span>
            <Activity size={20} />
            <strong>{endpointCount}</strong> listening endpoints
          </span>
          <span>
            <strong>
              {inventory?.processes.filter((p) => p.endpoints.length).length ??
                0}
            </strong>{" "}
            listening apps
          </span>
          <span>
            <ShieldCheck size={18} /> System processes protected
          </span>
        </div>
        <p>
          Find local previews and apps using ports, including 127.x.x.x,
          localhost, IPv6 and all-interface listeners. UDP entries are bound
          endpoints; they do not have TCP’s listening state.
        </p>
        <div className="toolbar-actions">
          <button disabled={working || loading} onClick={() => void refresh()}>
            <RefreshCw size={16} />
            Refresh processes
          </button>
          <label className="cloud-check">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            Refresh every 5 seconds
          </label>
          <label className="cloud-check">
            <input
              type="checkbox"
              checked={secure}
              onChange={(e) => setSecure(e.target.checked)}
            />
            Use HTTPS for previews
          </label>
        </div>
        <small>
          {inventory
            ? `Last checked ${new Date(inventory.scannedAt).toLocaleTimeString()}. `
            : ""}
          Automatic refresh runs only while this tab is open and visible.
          Previews open in your browser; a TCP port may serve something other
          than a website.
        </small>
      </section>
      <OperationStatus
        busy={loading || working}
        message={
          working
            ? "Completing process action…"
            : "Reading Windows listeners and apps…"
        }
        success={notice}
      />
      {(readError || error) && (
        <div role="alert" className="banner error">
          {readError || error}
          {readError && " Actions are paused until Refresh succeeds."}
        </div>
      )}
      {inventory?.warnings.map((warning) => (
        <div role="alert" className="banner error" key={warning}>
          {warning} The list may be incomplete.
        </div>
      ))}
      {results.length > 0 && (
        <section className="panel process-results" aria-label="Stop results">
          <h2>
            {results.filter((r) => r.stopped).length} of {results.length}{" "}
            processes stopped
          </h2>
          <ul>
            {results.map((r) => (
              <li key={r.pid}>
                PID {r.pid}: {r.message}
              </li>
            ))}
          </ul>
          <button onClick={() => setResults([])}>Dismiss results</button>
        </section>
      )}
      <section className="panel process-list" aria-label="Running processes">
        <div className="process-filters">
          <label>
            Search apps or ports
            <input
              type="search"
              value={search}
              placeholder="Name, PID, path or port…"
              onChange={(e) => {
                setSearch(e.target.value);
                filterChanged();
              }}
            />
          </label>
          <label>
            Show
            <select
              value={mode}
              onChange={(e) => {
                setMode(e.target.value);
                filterChanged();
              }}
            >
              <option value="listeners">Listening apps</option>
              <option value="all">User apps and listeners</option>
            </select>
          </label>
          <label>
            Protocol
            <select
              value={protocol}
              onChange={(e) => {
                setProtocol(e.target.value);
                filterChanged();
              }}
            >
              <option value="all">TCP + UDP</option>
              <option value="TCP">TCP</option>
              <option value="UDP">UDP</option>
            </select>
          </label>
          <label>
            Interface
            <select
              value={scope}
              onChange={(e) => {
                setScope(e.target.value);
                filterChanged();
              }}
            >
              <option value="all">All interfaces</option>
              <option value="local">Loopback + wildcard</option>
            </select>
          </label>
        </div>
        <div className="process-bulk toolbar-actions">
          <button
            disabled={disabled || !eligible.length}
            onClick={() => setSelected(new Set(eligible.map(processKey)))}
          >
            Select all shown
          </button>
          <button
            disabled={!selected.size || working}
            onClick={() => setSelected(new Set())}
          >
            Clear selection
          </button>
          <button
            className="danger"
            disabled={disabled || !chosen.length}
            onClick={() => setReview(chosen)}
          >
            <Square size={14} />
            Stop selected ({chosen.length})
          </button>
          <button
            className="danger"
            disabled={disabled || !eligible.length}
            onClick={() => setReview(eligible)}
          >
            <Square size={14} />
            Stop all shown ({eligible.length})
          </button>
        </div>
        <p className="process-help">
          Bulk actions cover the current filters across all pages. Protected
          apps are excluded. Stop ends the whole owning process and all of its
          ports, including ports hidden by a filter.
        </p>
        <div className="table-scroll">
          <table className="process-table">
            <thead>
              <tr>
                <th scope="col">Select</th>
                <th scope="col">App / process</th>
                <th scope="col">Ports and preview</th>
                <th scope="col">Actions</th>
              </tr>
            </thead>
            <tbody>
              {rows.map(({ process: p, endpoints }) => (
                <tr key={processKey(p)}>
                  <td>
                    <input
                      type="checkbox"
                      aria-label={`Select ${p.name} PID ${p.pid}`}
                      disabled={disabled || !p.canStop}
                      checked={selected.has(processKey(p))}
                      onChange={(e) =>
                        setSelected((previous) => {
                          const next = new Set(previous);
                          if (e.target.checked) next.add(processKey(p));
                          else next.delete(processKey(p));
                          return next;
                        })
                      }
                    />
                  </td>
                  <td>
                    <strong>{p.name}</strong>
                    <small>
                      PID {p.pid} · Parent {p.parentPid}
                    </small>
                    <span className="process-path">
                      {p.path || "Executable path unavailable"}
                    </span>
                    <small>
                      {p.startedAt
                        ? `Started ${new Date(p.startedAt).toLocaleString()}`
                        : "Start time unavailable"}
                    </small>
                    {!p.canStop && (
                      <small className="process-protection">
                        {p.protection}
                      </small>
                    )}
                  </td>
                  <td>
                    {endpoints.length ? (
                      endpoints.map((e) => (
                        <div
                          className="process-endpoint"
                          key={endpointLabel(e)}
                        >
                          <code>{endpointLabel(e)}</code>
                          <div>
                            <button
                              aria-label={`Copy ${endpointLabel(e)}`}
                              title="Copy endpoint"
                              disabled={working}
                              onClick={() =>
                                void action(
                                  () =>
                                    navigator.clipboard.writeText(
                                      endpointLabel(e),
                                    ),
                                  "Endpoint copied.",
                                )
                              }
                            >
                              <Copy size={14} />
                            </button>
                            {e.protocol === "TCP" &&
                              isLocalEndpoint(e) &&
                              e.port > 0 && (
                                <button
                                  disabled={disabled || !p.started}
                                  aria-label={`Preview ${p.name} port ${e.port} at ${e.address}`}
                                  onClick={() =>
                                    void action(() =>
                                      processes.preview(p, e, secure),
                                    )
                                  }
                                >
                                  <ExternalLink size={14} />
                                  Preview
                                </button>
                              )}
                          </div>
                        </div>
                      ))
                    ) : (
                      <small>No listening ports</small>
                    )}
                  </td>
                  <td>
                    <div className="process-row-actions">
                      <button
                        disabled={disabled || !p.canStop}
                        className="danger"
                        aria-label={`Stop ${p.name} PID ${p.pid}`}
                        onClick={() => setReview([p])}
                      >
                        <Square size={14} />
                        Stop
                      </button>
                      <button
                        disabled={working}
                        aria-label={`Copy details for ${p.name} PID ${p.pid}`}
                        onClick={() =>
                          void action(
                            () =>
                              navigator.clipboard.writeText(
                                `${p.name}\nPID ${p.pid} · Parent ${p.parentPid}\n${p.path}\n${p.endpoints.map(endpointLabel).join("\n")}`,
                              ),
                            "Process details copied.",
                          )
                        }
                      >
                        <Copy size={14} />
                        Details
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {inventory && !visible.length && (
          <p className="process-help">No processes match these filters.</p>
        )}
        <div className="process-pagination">
          <span>
            {visible.length} apps · Page {currentPage + 1} of {pageCount}
          </span>
          <button
            disabled={currentPage === 0}
            onClick={() => setPage(currentPage - 1)}
          >
            Previous
          </button>
          <button
            disabled={currentPage + 1 >= pageCount}
            onClick={() => setPage(currentPage + 1)}
          >
            Next
          </button>
        </div>
      </section>
      <section
        className="panel process-startup"
        aria-label="Startup diagnostics"
      >
        <h2>Returning after a restart?</h2>
        <p>
          Stopping an app ends this instance. Startup entries, scheduled tasks,
          services or a parent app can launch it again. ContextDock never stops
          apps automatically.
        </p>
        <div className="toolbar-actions">
          <button
            disabled={working}
            onClick={() =>
              void action(async () => {
                const items = await processes.startup();
                if (active.current) setStartup(items);
              })
            }
          >
            Inspect startup entries
          </button>
          <button
            disabled={working}
            onClick={() => void action(processes.openStartup)}
          >
            <ExternalLink size={16} />
            Windows Startup apps
          </button>
        </div>
        {startup && (
          <div className="process-startup-items">
            <p>
              Registered Run / RunOnce entries and Startup folder items. Entries
              may be disabled in Windows. This does not prove which item
              launched a process; scheduled tasks and services are not included.
            </p>
            {startup.length ? (
              <ul>
                {startup.map((item, index) => (
                  <li key={`${item.source}:${item.name}:${index}`}>
                    <strong>{item.name}</strong>
                    <small>{item.source}</small>
                  </li>
                ))}
              </ul>
            ) : (
              <p>No entries found in these locations.</p>
            )}
            <button onClick={() => setStartup(null)}>
              Hide startup entries
            </button>
          </div>
        )}
      </section>
      {review && (
        <Modal
          title="Stop these processes?"
          close={() => {
            if (!acting.current) setReview(null);
          }}
        >
          <p>
            This force stops {review.length} whole{" "}
            {review.length === 1 ? "process" : "processes"}. Unsaved work and
            active requests can be lost. Child processes are not automatically
            stopped; a parent app may restart a stopped process.
          </p>
          <ul className="process-review">
            {review.map((p) => (
              <li key={processKey(p)}>
                <strong>
                  {p.name} · PID {p.pid}
                </strong>
                <small>{p.path}</small>
                <small>
                  {p.endpoints.map(endpointLabel).join(", ") ||
                    "No listening ports"}
                </small>
              </li>
            ))}
          </ul>
          <p>
            Windows services, other user sessions and ContextDock’s related
            processes are protected. Each process identity is checked again
            before stopping.
          </p>
          {error && <p role="alert">{error}</p>}
          <div className="modal-actions">
            <button disabled={working} onClick={() => setReview(null)}>
              Cancel
            </button>
            <button
              className="danger"
              disabled={working || review.length > 256}
              onClick={() => void stopReviewed()}
            >
              {working
                ? "Stopping…"
                : `Force stop ${review.length} ${review.length === 1 ? "process" : "processes"}`}
            </button>
          </div>
          {review.length > 256 && (
            <p role="alert">Select at most 256 processes at a time.</p>
          )}
        </Modal>
      )}
    </div>
  );
}
