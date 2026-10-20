import { useEffect, useRef, useState } from "react";
import {
  ai,
  MINI,
  QUALITY,
  type AnalysisRequest,
  type Baseline,
  type Goals,
  type IntelligenceView,
  type Preview,
  type Report,
  type Citation,
} from "../ai";
import { date } from "../format";
import { registry } from "../services/registry";
import { VersionComparison } from "./VersionComparison";
import type { Version } from "../types";
const empty: Goals = {
  purpose: "",
  goals: "",
  requirements: "",
  nonGoals: "",
  constraints: "",
  deliverables: "",
  principles: "",
};
const labels: Record<keyof Goals, string> = {
  purpose: "Project purpose",
  goals: "Original goals",
  requirements: "Primary requirements",
  nonGoals: "Non-goals / excluded scope",
  constraints: "Major constraints",
  deliverables: "Expected deliverables",
  principles: "Architectural principles",
};
export function ProjectIntelligence({
  project,
  name,
  initialPair,
}: {
  project: string;
  name: string;
  initialPair?: [string, string];
}) {
  const [view, setView] = useState<IntelligenceView | null>(null);
  const [request, setRequest] = useState<AnalysisRequest>({
    kind: "compare",
    versions: initialPair ?? [],
    baseline: null,
    model: MINI,
    inputBudget: 32768,
    outputTokens: 4096,
  });
  const [family, setFamily] = useState("");
  const [pair, setPair] = useState<[string, string]>(initialPair ?? ["", ""]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [report, setReport] = useState<Report | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [consent, setConsent] = useState(false);
  const [fresh, setFresh] = useState(false);
  const [notice, setNotice] = useState("");
  const [editor, setEditor] = useState(false);
  const [title, setTitle] = useState("");
  const [goals, setGoals] = useState<Goals>(empty);
  const [sourceVersion, setSourceVersion] = useState("");
  const [approved, setApproved] = useState(false);
  const [inspection, setInspection] = useState<{
    document: string;
    pair: [string, string];
    versions: Version[];
  } | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    void ai
      .action<IntelligenceView>(project, "view")
      .then((v) => {
        if (!alive.current) return;
        setView(v);
        setRequest((r) => ({
          ...r,
          baseline: v.baselines.find((b) => b.active)?.id ?? null,
        }));
        if (initialPair) {
          setPair(initialPair);
          setFamily(
            v.versions.find((v) => v.id === initialPair[0])?.document ?? "",
          );
        }
      })
      .catch((e) => {
        if (alive.current) setError(String(e));
      });
    return () => {
      alive.current = false;
    };
  }, [project, initialPair]);
  async function load() {
    const v = await ai.action<IntelligenceView>(project, "view");
    if (alive.current) setView(v);
  }
  async function run(work: () => Promise<void>) {
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
        void load().catch(() => {});
      }
    }
  }
  function edit(next: Partial<AnalysisRequest>) {
    setRequest((r) => ({ ...r, ...next }));
    setPreview(null);
    setReport(null);
  }
  const families = [
    ...new Map(
      (view?.versions ?? []).map((v) => [v.document, v.name]),
    ).entries(),
  ];
  const versions = (view?.versions ?? []).filter((v) => v.document === family);
  const active = view?.baselines.find((b) => b.active);
  function baselineEditor(b?: Baseline) {
    setTitle(b ? `${b.title} — revised` : "");
    setGoals(b ? (JSON.parse(b.content) as Goals) : empty);
    setSourceVersion(b?.source_version ?? "");
    setApproved(false);
    setEditor(true);
  }
  async function openReport(id: string) {
    const next = await ai.action<Report>(project, "report", { id });
    if (alive.current) {
      setReport(next);
      setPreview(null);
    }
  }
  async function inspect(version: string) {
    if (!report) return;
    const source = report.plan.sources.find((v) => v.version === version);
    if (!source) return;
    const transition = report.plan.transitions.find(
      (t) => t.earlier === version || t.later === version,
    );
    const baseline = report.plan.baseline?.source;
    const selected: [string, string] = transition
      ? [transition.earlier, transition.later]
      : baseline?.document === source.document
        ? [baseline.version, version]
        : [version, version];
    const versions = await registry.versions(source.document);
    if (alive.current)
      setInspection({ document: source.document, pair: selected, versions });
  }
  function citations(refs: Citation[]) {
    return refs.map((ref, i) => {
      const e = report?.plan.evidence.find((e) => e.id === ref.evidenceId);
      const v = report?.plan.sources.find((v) => v.version === e?.version);
      return (
        <details key={`${ref.evidenceId}:${i}`}>
          <summary>
            {v
              ? `${v.name} · v${v.number}`
              : `Baseline revision ${report?.plan.baseline?.revision}`}{" "}
            · lines {e?.start}–{e?.end} · {e?.heading || "Source evidence"}
          </summary>
          <blockquote>{ref.quote}</blockquote>
          <pre className="ai-evidence">{e?.text}</pre>
          {v && (
            <button onClick={() => void run(() => inspect(v.version))}>
              Open factual comparison
            </button>
          )}
        </details>
      );
    });
  }
  if (inspection)
    return (
      <VersionComparison
        documentId={inspection.document}
        versions={inspection.versions}
        initialPair={inspection.pair}
        back={() => setInspection(null)}
      />
    );
  return (
    <section className="panel intelligence" aria-label="Project intelligence">
      <h2>Project intelligence</h2>
      <button disabled={busy} onClick={() => void run(load)}>
        Refresh intelligence · local only
      </button>
      <p>
        {name} · AI interpretation is optional. Snapshots and local diffs remain
        authoritative.
      </p>
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {busy && (
        <p role="status">
          Working… Local tracking and cloud sync remain independent.
        </p>
      )}
      {!view ? (
        <p>Loading local baselines and saved reports…</p>
      ) : (
        <>
          <div className="ai-toolbar">
            <strong>
              Approved baseline:{" "}
              {active
                ? `${active.title} · revision ${active.revision}`
                : "None"}
            </strong>
            <button disabled={busy} onClick={() => baselineEditor()}>
              Create baseline
            </button>
            {active && (
              <button disabled={busy} onClick={() => baselineEditor(active)}>
                Revise current baseline
              </button>
            )}
          </div>
          {view.baselines.map((b) => (
            <details key={b.id}>
              <summary>
                {b.title} · revision {b.revision}{" "}
                {b.active ? "· Current approved" : "· Historical"}
              </summary>
              <p>
                Approved {date(b.approved_at)} · Source{" "}
                {b.source_version
                  ? "captured snapshot (hash retained)"
                  : "manual"}
              </p>
              {Object.entries(JSON.parse(b.content) as Goals).map(([k, v]) => (
                <p key={k}>
                  <strong>{labels[k as keyof Goals]}:</strong>{" "}
                  {v || "Not specified"}
                </p>
              ))}
            </details>
          ))}
          {editor && (
            <fieldset disabled={busy} className="ai-editor">
              <legend>
                Approve a new baseline revision · local only, no API call
              </legend>
              <label>
                Baseline title
                <input
                  value={title}
                  maxLength={160}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </label>
              <label>
                Baseline source
                <select
                  value={sourceVersion}
                  onChange={(e) => setSourceVersion(e.target.value)}
                >
                  <option value="">Manual baseline</option>
                  {view.versions.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.name} · v{v.number} · {date(v.capturedAt)}
                    </option>
                  ))}
                </select>
              </label>
              {sourceVersion && (
                <button
                  onClick={() =>
                    void run(async () => {
                      const source = await ai.action<{
                        text: string;
                        warnings: string[];
                      }>(project, "source", { version: sourceVersion });
                      if (alive.current) {
                        setGoals((g) => ({ ...g, purpose: source.text }));
                        setNotice(
                          `Verified snapshot text copied into Project purpose. Edit into goals and review before approval. ${source.warnings.join(" ")}`,
                        );
                      }
                    })
                  }
                >
                  Use verified snapshot text as starting point
                </button>
              )}
              {(Object.keys(labels) as (keyof Goals)[]).map((k) => (
                <label key={k}>
                  {labels[k]}
                  <textarea
                    value={goals[k]}
                    onChange={(e) =>
                      setGoals((g) => ({ ...g, [k]: e.target.value }))
                    }
                  />
                </label>
              ))}
              <label className="cloud-check">
                <input
                  type="checkbox"
                  checked={approved}
                  onChange={(e) => setApproved(e.target.checked)}
                />
                I approve these objectives as a new baseline revision.
              </label>
              <div className="ai-toolbar">
                <button
                  disabled={!approved || !title.trim() || !goals.purpose.trim()}
                  onClick={() =>
                    void run(async () => {
                      const id = await ai.action<string>(project, "approve", {
                        title,
                        content: goals,
                        sourceVersion: sourceVersion || null,
                        supersedes: active?.id ?? null,
                        approve: true,
                      });
                      if (alive.current) {
                        setEditor(false);
                        edit({ baseline: id });
                        setNotice(
                          "Baseline approved locally. Previous revisions and reports are preserved.",
                        );
                      }
                    })
                  }
                >
                  Approve baseline
                </button>
                <button onClick={() => setEditor(false)}>
                  Cancel baseline
                </button>
              </div>
            </fieldset>
          )}
          {!preview && !report && (
            <fieldset disabled={busy}>
              <legend>Choose evidence for an on-demand paid analysis</legend>
              <label>
                Analysis type
                <select
                  value={request.kind}
                  onChange={(e) =>
                    edit({
                      kind: e.target.value as AnalysisRequest["kind"],
                      versions: [],
                    })
                  }
                >
                  <option value="compare">Analyze Changes</option>
                  <option value="alignment">Review Goal Alignment</option>
                  <option value="timeline">Generate Evolution Timeline</option>
                  <option value="consistency">
                    Review Project Consistency
                  </option>
                </select>
              </label>
              {["compare", "timeline"].includes(request.kind) ? (
                <>
                  <label>
                    Document family
                    <select
                      value={family}
                      onChange={(e) => {
                        setFamily(e.target.value);
                        setPair(["", ""]);
                      }}
                    >
                      <option value="">Choose a family…</option>
                      {families.map(([id, name]) => (
                        <option key={id} value={id}>
                          {name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="comparison-selectors">
                    {[0, 1].map((i) => (
                      <label key={i}>
                        {i === 0
                          ? "Earlier / range start"
                          : "Later / range end"}
                        <select
                          value={pair[i]}
                          onChange={(e) =>
                            setPair((old) =>
                              i === 0
                                ? [e.target.value, old[1]]
                                : [old[0], e.target.value],
                            )
                          }
                        >
                          <option value="">Choose a version…</option>
                          {versions.map((v) => (
                            <option key={v.id} value={v.id}>
                              v{v.number} · {date(v.capturedAt)}
                            </option>
                          ))}
                        </select>
                      </label>
                    ))}
                  </div>
                  {request.kind === "timeline" && (
                    <p>
                      Every intermediate capture is included, in order. Maximum
                      12 captures; one planned model request.
                    </p>
                  )}
                </>
              ) : (
                <div className="ai-version-list">
                  <p>
                    Explicitly select the versions to review. For consistency,
                    choose one version from each of 2–6 families.
                  </p>
                  {view.versions.map((v) => (
                    <label className="cloud-check" key={v.id}>
                      <input
                        type="checkbox"
                        checked={request.versions.includes(v.id)}
                        onChange={(e) =>
                          edit({
                            versions: e.target.checked
                              ? [...request.versions, v.id]
                              : request.versions.filter((id) => id !== v.id),
                          })
                        }
                      />
                      {v.name} · v{v.number} · {date(v.capturedAt)}{" "}
                      {v.current ? "· current" : "· historical"}
                    </label>
                  ))}
                </div>
              )}
              <label>
                Approved baseline revision
                <select
                  value={request.baseline ?? ""}
                  onChange={(e) => edit({ baseline: e.target.value || null })}
                >
                  <option value="">No baseline (required for alignment)</option>
                  {view.baselines.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.title} · revision {b.revision}
                      {b.active ? " · current" : " · historical"}
                    </option>
                  ))}
                </select>
              </label>
              <div className="comparison-selectors">
                <label>
                  Analysis model
                  <select
                    value={request.model}
                    onChange={(e) => edit({ model: e.target.value })}
                  >
                    <option value={MINI}>
                      GPT-5.4 Mini · lower cost default
                    </option>
                    <option value={QUALITY}>
                      GPT-5.4 · higher cost, more capable
                    </option>
                  </select>
                </label>
                <label>
                  Input evidence budget (conservative units)
                  <input
                    type="number"
                    min={4096}
                    max={65536}
                    value={request.inputBudget}
                    onChange={(e) =>
                      edit({ inputBudget: Number(e.target.value) })
                    }
                  />
                </label>
                <label>
                  Maximum output tokens
                  <input
                    type="number"
                    min={512}
                    max={8192}
                    value={request.outputTokens}
                    onChange={(e) =>
                      edit({ outputTokens: Number(e.target.value) })
                    }
                  />
                </label>
              </div>
              <button
                onClick={() =>
                  void run(async () => {
                    const selected = ["compare", "timeline"].includes(
                      request.kind,
                    )
                      ? pair
                      : request.versions;
                    const next = await ai.action<Preview>(project, "prepare", {
                      request: { ...request, versions: selected },
                    });
                    if (alive.current) {
                      setPreview(next);
                      setConsent(false);
                      setFresh(false);
                    }
                  })
                }
              >
                Preview analysis plan · no model call
              </button>
            </fieldset>
          )}
          {preview && (
            <div
              className="ai-preview"
              role="region"
              aria-label="Analysis request preview"
            >
              <h3>Review paid analysis · {preview.plan.request.kind}</h3>
              <p>
                {name} · {preview.plan.request.model} · {preview.requests} model
                request · No vector retrieval or new cloud file uploads.
              </p>
              <ul>
                {preview.plan.sources.map((v) => (
                  <li key={v.version}>
                    {v.name} · v{v.number} · captured {date(v.capturedAt)} ·
                    SHA-256 verified
                  </li>
                ))}
              </ul>
              <p>
                Baseline:{" "}
                {preview.plan.baseline
                  ? `${preview.plan.baseline.title} · revision ${preview.plan.baseline.revision}`
                  : "None"}
              </p>
              <p>{preview.plan.scope}</p>
              {preview.plan.warnings.map((w, i) => (
                <p key={i}>{w}</p>
              ))}
              <p>
                About {preview.plan.estimatedTokens.toLocaleString()} input
                tokens (heuristic, not tokenizer output);{" "}
                {preview.plan.inputUnits.toLocaleString()} conservative input
                units. Maximum {preview.plan.request.outputTokens} output
                tokens.
              </p>
              <p>
                Estimated USD ${preview.plan.estimateUsd.toFixed(4)};
                conservative planning estimate $
                {preview.plan.conservativeUsd.toFixed(4)}. Not an account
                billing total or guaranteed dollar cap. Pricing:{" "}
                {preview.plan.pricingDate}.
              </p>
              <p>
                Only the evidence listed above and approved baseline text will
                be sent to OpenAI. Responses use store:false; this does not
                override all provider retention or processing policies. Reports
                and evidence are saved locally. Existing indexed files follow
                their separate cloud lifecycle.
              </p>
              {preview.cached && (
                <>
                  <button
                    disabled={busy}
                    onClick={() => void run(() => openReport(preview.cached!))}
                  >
                    View Saved Analysis · no API call
                  </button>
                  <label className="cloud-check">
                    <input
                      type="checkbox"
                      checked={fresh}
                      disabled={busy}
                      onChange={(e) => setFresh(e.target.checked)}
                    />
                    Generate a fresh paid analysis instead of the saved result
                  </label>
                </>
              )}
              <label className="cloud-check">
                <input
                  type="checkbox"
                  disabled={busy}
                  checked={consent}
                  onChange={(e) => setConsent(e.target.checked)}
                />
                I approve sending this evidence for one potentially paid
                analysis.
              </label>
              <div className="ai-toolbar">
                <button
                  className="primary"
                  disabled={busy || !consent || (!!preview.cached && !fresh)}
                  onClick={() =>
                    void run(async () => {
                      const result = await ai.action<Report>(
                        project,
                        "generate",
                        { id: preview.id, consent: true, fresh },
                      );
                      if (alive.current) {
                        setReport(result);
                        setPreview(null);
                      }
                    })
                  }
                >
                  Generate Analysis · paid API
                </button>
                <button
                  onClick={() => {
                    void ai
                      .action(project, "cancel", { id: preview.id })
                      .then(() => {
                        if (alive.current) {
                          setNotice(
                            busy
                              ? "Cancellation requested. A sent request may finish and incur charges; a valid completed result will still be saved."
                              : "Unsent analysis cancelled.",
                          );
                          if (!busy) setPreview(null);
                        }
                      })
                      .catch((e) => {
                        if (alive.current) setError(String(e));
                      });
                  }}
                >
                  {busy ? "Cancel pending analysis" : "Cancel analysis"}
                </button>
              </div>
            </div>
          )}
          {report && (
            <div className="ai-report">
              <div className="ai-toolbar">
                <h3>
                  {report.kind} · {report.state}
                </h3>
                <button disabled={busy} onClick={() => setReport(null)}>
                  New analysis
                </button>
              </div>
              <p>
                Saved {date(report.completed_at ?? report.created_at)} ·{" "}
                {report.plan.request.model} · prompt {report.plan.promptVersion}{" "}
                / schema {report.plan.schemaVersion}
              </p>
              <p>
                Exact selected baseline:{" "}
                {report.plan.baseline
                  ? `${report.plan.baseline.title} · revision ${report.plan.baseline.revision}`
                  : "None"}
                . This report preserves historical inputs; it does not update
                automatically.
              </p>
              <p>{report.plan.scope}</p>
              {report.error && <p role="alert">{report.error}</p>}
              <p>
                Returned usage: {report.usage?.input_tokens ?? "Not returned"}{" "}
                input tokens · {report.usage?.output_tokens ?? "Not returned"}{" "}
                output tokens.{" "}
                {report.usage?.input_tokens !== undefined &&
                  report.usage.output_tokens !== undefined &&
                  `Estimated usage cost USD $${((report.usage.input_tokens * (report.plan.request.model === MINI ? 0.75 : 2.5) + report.usage.output_tokens * (report.plan.request.model === MINI ? 4.5 : 15)) / 1e6).toFixed(5)} at saved standard rates; not official billing.`}
              </p>
              {report.result && (
                <>
                  <p>
                    <strong>
                      AI interpretation · verify against evidence.
                    </strong>{" "}
                    Review categories are qualitative interpretations, not
                    objective scores.
                  </p>
                  <p>{report.result.summary}</p>
                  {citations(report.result.summaryRefs)}
                  <ol
                    className={
                      report.kind === "timeline" ? "ai-timeline" : "ai-findings"
                    }
                  >
                    {report.result.findings.map((f, i) => {
                      const event = report.plan.sources.find(
                        (v) => v.version === f.atVersion,
                      );
                      return (
                        <li key={i}>
                          <h4>
                            {f.category.replaceAll("_", " ")} · {f.title}
                          </h4>
                          {event && (
                            <p>
                              v{event.number} · {date(event.capturedAt)}
                            </p>
                          )}
                          <p>{f.detail}</p>
                          {citations(f.refs)}
                          <label>
                            Your review decision
                            <select
                              disabled={busy}
                              value={
                                report.decisions.find((d) => d.finding === i)
                                  ?.decision ?? "unreviewed"
                              }
                              onChange={(e) => {
                                const decision = e.target.value;
                                void run(async () => {
                                  await ai.action(project, "decision", {
                                    id: report.id,
                                    finding: i,
                                    decision,
                                  });
                                  await openReport(report.id);
                                });
                              }}
                            >
                              <option value="unreviewed">Unreviewed</option>
                              <option value="reviewed">Reviewed</option>
                              <option value="intentional">
                                Intentional change
                              </option>
                              <option value="follow_up">Needs follow-up</option>
                              <option value="dismissed">Dismissed</option>
                            </select>
                          </label>
                        </li>
                      );
                    })}
                  </ol>
                </>
              )}
            </div>
          )}
          <details>
            <summary>
              Saved analyses and request history ({view.reports.length}, latest
              100)
            </summary>
            {view.reports.map((r) => (
              <div className="ai-toolbar" key={r.id}>
                <span>
                  {r.kind} · {r.state} · {date(r.completed_at ?? r.created_at)}
                </span>
                <button
                  disabled={busy}
                  onClick={() => void run(() => openReport(r.id))}
                >
                  Open saved report
                </button>
              </div>
            ))}
          </details>
        </>
      )}
    </section>
  );
}
