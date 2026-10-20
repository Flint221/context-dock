import { useEffect, useMemo, useRef, useState } from "react";
import { registry } from "../services/registry";
import type { Version } from "../types";
import type { Comparison, DiffLine } from "../comparison";
import { date } from "../format";
import { OperationStatus } from "./OperationStatus";

const PAGE_SIZE = 100;
function label(v: Version) {
  return `v${v.number} · ${date(v.capturedAt)} · ${v.sourceName}`;
}
type Row = { left: DiffLine | null; right: DiffLine | null };
// Pair replacement regions; a single-sided insertion/removal keeps the opposite side blank.
function align(lines: DiffLine[]): Row[] {
  const rows: Row[] = [];
  let removed: DiffLine[] = [],
    added: DiffLine[] = [];
  const flush = () => {
    for (let i = 0; i < Math.max(removed.length, added.length); i++)
      rows.push({ left: removed[i] || null, right: added[i] || null });
    removed = [];
    added = [];
  };
  for (const line of lines) {
    if (line.kind === "equal") {
      flush();
      rows.push({ left: line, right: line });
    } else if (line.kind === "remove") removed.push(line);
    else added.push(line);
  }
  flush();
  return rows;
}
function Cell({ line, side }: { line: DiffLine | null; side: "old" | "new" }) {
  return (
    <div className={`diff-cell ${line?.kind || "blank"}`}>
      <span className="line-number">
        {line ? (side === "old" ? line.oldLine : line.newLine) : ""}
      </span>
      <span className="diff-sign" aria-label={line?.kind}>
        {line?.kind === "add" ? "+" : line?.kind === "remove" ? "−" : " "}
      </span>
      <code>
        {line?.text}
        {line && !line.terminated && (
          <span className="no-newline"> ⏎ No final newline</span>
        )}
      </code>
    </div>
  );
}
export function VersionComparison({
  documentId,
  versions,
  back,
  initialPair,
  analyze,
}: {
  documentId: string;
  versions: Version[];
  back: () => void;
  initialPair?: [string, string];
  analyze?: (pair: [string, string]) => void;
}) {
  const ordered = useMemo(
    () => [...versions].sort((a, b) => b.number - a.number),
    [versions],
  );
  const [left, setLeft] = useState(
    initialPair?.[0] ?? ordered[1]?.id ?? ordered[0]?.id ?? "",
  );
  const [right, setRight] = useState(initialPair?.[1] ?? ordered[0]?.id ?? "");
  const [request, setRequest] = useState<{
    left: string;
    right: string;
    revision: number;
  } | null>(null);
  const [result, setResult] = useState<Comparison | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState("unified");
  const [change, setChange] = useState(0);
  const [page, setPage] = useState(0);
  const [context, setContext] = useState(false);
  const [notice, setNotice] = useState("");
  const content = useRef<HTMLElement>(null);
  const backButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    backButton.current?.focus();
  }, []);
  // Explicit requests avoid silently re-running comparisons after every watcher event.
  useEffect(() => {
    if (!request) return;
    let active = true;
    registry
      .compare(request.left, request.right)
      .then((value) => {
        if (!active) return;
        if (value.documentId !== documentId)
          throw new Error(
            "Comparison identity conflict. Return to history and retry.",
          );
        setResult(value);
        setChange(0);
        setPage(0);
        setContext(false);
        setLoading(false);
      })
      .catch((e) => {
        if (active) {
          setError(String(e));
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [request, documentId]);
  const selected = ordered.find((v) => v.id === left);
  const other = ordered.find((v) => v.id === right);
  const reverse = !!selected && !!other && selected.number > other.number;
  const indexes = useMemo(
    () =>
      result?.blocks.flatMap((b, i) => (b.kind === "change" ? [i] : [])) || [],
    [result],
  );
  const blockIndex = indexes[change] ?? 0;
  const block = result?.blocks[blockIndex];
  const lines = useMemo(() => {
    if (!result || !block) return [];
    if (block.kind === "equal") return block.lines;
    const before = result.blocks[blockIndex - 1];
    const after = result.blocks[blockIndex + 1];
    return [
      ...(before?.kind === "equal"
        ? context
          ? before.lines
          : before.lines.slice(-3)
        : []),
      ...block.lines,
      ...(after?.kind === "equal"
        ? context
          ? after.lines
          : after.lines.slice(0, 3)
        : []),
    ];
  }, [result, block, blockIndex, context]);
  const rows = useMemo(
    () => (mode === "split" ? align(lines) : []),
    [lines, mode],
  );
  const count = mode === "split" ? rows.length : lines.length;
  const pages = Math.max(1, Math.ceil(count / PAGE_SIZE));
  const visiblePage = Math.min(page, pages - 1);
  const start = visiblePage * PAGE_SIZE;
  function showPage(next: number) {
    setPage(next);
    if (content.current) content.current.scrollTop = 0;
  }
  function navigate(next: number) {
    setChange(next);
    showPage(0);
    setContext(false);
    content.current?.scrollIntoView({ block: "nearest" });
  }
  async function copy(summary: boolean) {
    if (!result) return;
    try {
      const value = summary
        ? `${result.documentName}: v${result.earlier.number} → v${result.later.number}. ${result.addedLines} lines added, ${result.removedLines} removed, ${result.changedBlocks} change blocks. Text ${result.textEquivalent ? "equivalent" : "differs"}; bytes ${result.bytesIdentical ? "identical" : "differ"}.`
        : result.blocks
            .filter((b) => b.kind === "change")
            .flatMap((b) =>
              b.lines.map(
                (l) =>
                  `${l.kind === "add" ? "+" : "-"}${l.text}${l.terminated ? "" : " [No final newline]"}`,
              ),
            )
            .join("\n");
      await navigator.clipboard.writeText(value);
      setNotice("Copied to clipboard.");
    } catch (e) {
      setNotice(`Clipboard unavailable: ${String(e)}`);
    }
  }
  return (
    <div className="comparison">
      <button ref={backButton} onClick={back}>
        ← Back to version history
      </button>
      <p>
        Local snapshot comparison. Earlier capture on the left, later on the
        right. No AI analysis.
      </p>
      {analyze && (
        <button
          disabled={loading || !left || !right || left === right}
          onClick={() => analyze([left, right])}
        >
          Analyze These Changes with AI · paid preview
        </button>
      )}
      <div className="comparison-selectors">
        <label>
          Earlier version
          <select
            value={left}
            disabled={loading}
            onChange={(e) => {
              setLeft(e.target.value);
              setResult(null);
            }}
          >
            {ordered.map((v) => (
              <option key={v.id} value={v.id}>
                {label(v)}
              </option>
            ))}
          </select>
        </label>
        <label>
          Later version
          <select
            value={right}
            disabled={loading}
            onChange={(e) => {
              setRight(e.target.value);
              setResult(null);
            }}
          >
            {ordered.map((v) => (
              <option key={v.id} value={v.id}>
                {label(v)}
              </option>
            ))}
          </select>
        </label>
        <button
          className="primary"
          aria-busy={loading}
          disabled={loading || !selected || !other}
          onClick={() => {
            setError("");
            setNotice("");
            setResult(null);
            setLoading(true);
            setRequest((prior) => ({
              left,
              right,
              revision: (prior?.revision || 0) + 1,
            }));
          }}
        >
          {loading ? "Comparing…" : "Compare selected versions"}
        </button>
      </div>
      {reverse && (
        <p>
          Selections will be ordered by capture version number before
          comparison.
        </p>
      )}
      {left === right && (
        <p>
          The same captured version is selected; this verifies byte-identical
          content.
        </p>
      )}
      <OperationStatus
        busy={loading}
        message="Verifying snapshots and calculating differences…"
      />
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {notice && <p role="status">{notice}</p>}
      {result && (
        <>
          <div className="diff-summary" role="status">
            <strong>
              v{result.earlier.number} → v{result.later.number}
            </strong>
            <span>+{result.addedLines} lines added</span>
            <span>−{result.removedLines} lines removed</span>
            <span>{result.changedBlocks} change blocks</span>
            <span>Text {result.textEquivalent ? "equivalent" : "differs"}</span>
            <span>
              File bytes {result.bytesIdentical ? "identical" : "differ"}
            </span>
          </div>
          <details>
            <summary>Snapshot identity and extraction policy</summary>
            <p>
              Earlier: {result.earlier.sourceName} ·{" "}
              {date(result.earlier.capturedAt)} · {result.earlier.id}
            </p>
            <p className="hash">{result.earlier.sha256}</p>
            <p>
              Later: {result.later.sourceName} · {date(result.later.capturedAt)}{" "}
              · {result.later.id}
            </p>
            <p className="hash">{result.later.sha256}</p>
            <ul>
              {result.warnings.map((w, i) => (
                <li key={i}>{w}</li>
              ))}
            </ul>
          </details>
          {result.textEquivalent && (
            <p className="comparison-empty">
              No textual changes.{" "}
              {result.bytesIdentical
                ? "The preserved file bytes are identical."
                : "File bytes differ; normalization or DOCX package changes produced equivalent extracted text."}
            </p>
          )}
          <div className="diff-controls">
            <label>
              Display mode
              <select
                value={mode}
                onChange={(e) => {
                  setMode(e.target.value);
                  showPage(0);
                }}
              >
                <option value="unified">Unified diff</option>
                <option value="split">Side-by-side diff</option>
              </select>
            </label>
            <div
              className="diff-navigation"
              role="group"
              aria-label="Change navigation"
            >
              <button
                disabled={!indexes.length || change === 0}
                title={
                  !indexes.length
                    ? "No textual changes to navigate"
                    : change === 0
                      ? "Already at the first change"
                      : "Go to the previous change"
                }
                onClick={() => navigate(change - 1)}
              >
                Previous change
              </button>
              <span aria-live="polite">
                {indexes.length
                  ? `Change ${change + 1} of ${indexes.length}`
                  : "No changes"}
              </span>
              <button
                disabled={!indexes.length || change === indexes.length - 1}
                title={
                  !indexes.length
                    ? "No textual changes to navigate"
                    : change === indexes.length - 1
                      ? "Already at the last change"
                      : "Go to the next change"
                }
                onClick={() => navigate(change + 1)}
              >
                Next change
              </button>
            </div>
            <button
              className="diff-context-toggle"
              disabled={!indexes.length}
              onClick={() => {
                setContext(!context);
                showPage(0);
              }}
            >
              {context
                ? "Collapse unchanged context"
                : "Expand unchanged context"}
            </button>
          </div>
          <p className="diff-range">
            {block
              ? `Earlier lines ${block.oldStart} +${block.oldCount}; later lines ${block.newStart} +${block.newCount}.`
              : "Both versions are empty."}{" "}
            Showing {count ? start + 1 : 0}–{Math.min(start + PAGE_SIZE, count)}{" "}
            of {count} {mode === "split" ? "aligned rows" : "lines"} in this
            region.
          </p>
          <section
            ref={content}
            className="diff-content"
            aria-label="Comparison content"
          >
            {mode === "split" ? (
              <>
                <div className="diff-row diff-column-head">
                  <strong>Earlier · v{result.earlier.number}</strong>
                  <strong>Later · v{result.later.number}</strong>
                </div>
                {rows.slice(start, start + PAGE_SIZE).map((row, i) => (
                  <div className="diff-row" key={start + i}>
                    <Cell line={row.left} side="old" />
                    <Cell line={row.right} side="new" />
                  </div>
                ))}
              </>
            ) : (
              lines.slice(start, start + PAGE_SIZE).map((line, i) => (
                <div className={`unified-line ${line.kind}`} key={start + i}>
                  <span className="line-number">{line.oldLine}</span>
                  <Cell line={line} side="new" />
                </div>
              ))
            )}
          </section>
          <div className="diff-controls">
            <div
              className="diff-navigation"
              role="group"
              aria-label="Line pagination"
            >
              <button
                disabled={visiblePage === 0}
                onClick={() => showPage(visiblePage - 1)}
              >
                Previous lines
              </button>
              <span>
                Page {visiblePage + 1} of {pages}
              </span>
              <button
                disabled={visiblePage === pages - 1}
                onClick={() => showPage(visiblePage + 1)}
              >
                Next lines
              </button>
            </div>
            <div
              className="diff-copy-actions"
              role="group"
              aria-label="Copy comparison"
            >
              <button onClick={() => void copy(true)}>
                Copy comparison summary
              </button>
              <button
                disabled={!indexes.length}
                onClick={() => void copy(false)}
              >
                Copy changed text
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
