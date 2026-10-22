import { useEffect, useRef, useState } from "react";
import { Copy, FolderOpen } from "lucide-react";
import type { Family, Version } from "../types";
import { registry } from "../services/registry";
import { versionName, date, displayPath, size } from "../format";
import { Modal } from "./Modal";
import { VersionComparison } from "./VersionComparison";
export function DocumentHistory({
  family,
  close,
}: {
  family: Family;
  close: () => void;
  project?: string;
}) {
  const [versions, setVersions] = useState<Version[] | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [comparing, setComparing] = useState(false);
  const compareButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!comparing) compareButton.current?.focus();
  }, [comparing]);
  useEffect(() => {
    let active = true;
    registry
      .versions(family.id)
      .then((v) => {
        if (active) setVersions(v);
      })
      .catch((e) => {
        if (active) {
          setError(String(e));
          setVersions([]);
        }
      });
    return () => {
      active = false;
    };
  }, [family.id, family.versionCount, family.currentNumber]);
  async function action(work: () => Promise<unknown>, message = "") {
    setError("");
    setNotice("");
    try {
      await work();
      setNotice(message);
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <Modal
      title={`${family.name} — ${comparing ? "compare versions" : "version history"}`}
      close={close}
    >
      {comparing && versions ? (
        <VersionComparison
          documentId={family.id}
          versions={versions}
          back={() => setComparing(false)}
        />
      ) : (
        <>
          <p>
            Each filename version shows its latest saved revision. Earlier
            snapshots stay preserved locally. Opening a saved location verifies
            its SHA-256 hash first.
          </p>
          <div className="history-actions">
            <button
              disabled={!family.activeSourceId}
              onClick={() => void action(() => registry.openCurrent(family.id))}
            >
              <FolderOpen size={15} />
              Current document location
            </button>
            {versions && versions.length >= 2 ? (
              <button
                ref={compareButton}
                className="primary"
                onClick={() => setComparing(true)}
              >
                Compare Versions
              </button>
            ) : null}
          </div>
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
          {notice && <p role="status">{notice}</p>}
          {versions?.length === 1 ? (
            <p>
              Increase the filename version (for example v4 to v5) to compare
              versions. Identical file bytes reuse their existing captured
              version.
            </p>
          ) : null}
          {error && (
            <button
              onClick={() =>
                void action(async () =>
                  setVersions(await registry.versions(family.id)),
                )
              }
            >
              Reload saved versions
            </button>
          )}
          {versions === null ? (
            <p className="padded">Loading saved versions…</p>
          ) : versions.length === 0 ? (
            <p className="padded">No versions captured for this document.</p>
          ) : (
            <div className="version-list">
              {versions.map((v) => (
                <article className="version-card" key={v.id}>
                  <div className="section-heading">
                    <strong>{versionName(v.sourceName, v.number)}</strong>
                    <span className={`badge ${v.current ? "available" : ""}`}>
                      {v.current
                        ? "Current"
                        : v.archiveAvailable
                          ? "Archived"
                          : "Snapshot only"}
                    </span>
                    <time>{date(v.capturedAt)}</time>
                  </div>
                  <strong>{v.sourceName}</strong>
                  <p>
                    {v.reason.replaceAll("_", " ")} · {size(v.size)}
                  </p>
                  <div className="toolbar-actions">
                    <button
                      disabled={!v.snapshotAvailable}
                      onClick={() =>
                        void action(() => registry.openVersion(v.id, false))
                      }
                    >
                      <FolderOpen size={14} />
                      Snapshot location
                    </button>
                    <button
                      onClick={() =>
                        void action(
                          () =>
                            navigator.clipboard.writeText(
                              displayPath(v.snapshotPath),
                            ),
                          "Snapshot path copied.",
                        )
                      }
                    >
                      <Copy size={14} />
                      Copy snapshot path
                    </button>
                    {v.archivePath && (
                      <>
                        <button
                          disabled={!v.archiveAvailable}
                          onClick={() =>
                            void action(() => registry.openVersion(v.id, true))
                          }
                        >
                          <FolderOpen size={14} />
                          Archive location
                        </button>
                        <button
                          onClick={() =>
                            void action(
                              () =>
                                navigator.clipboard.writeText(
                                  displayPath(v.archivePath!),
                                ),
                              "Archive path copied.",
                            )
                          }
                        >
                          <Copy size={14} />
                          Copy archive path
                        </button>
                      </>
                    )}
                  </div>
                  <details>
                    <summary>Version metadata</summary>
                    <dl>
                      <dt>Original source</dt>
                      <dd>{v.sourceRelative}</dd>
                      <dt>Source modified</dt>
                      <dd>{date(v.modifiedAt)}</dd>
                      <dt>SHA-256</dt>
                      <dd className="hash">{v.sha256}</dd>
                      <dt>Immutable snapshot</dt>
                      <dd>{displayPath(v.snapshotPath)}</dd>
                      <dt>Availability</dt>
                      <dd>
                        Snapshot:{" "}
                        {v.snapshotAvailable ? "present" : "unavailable"} ·
                        Archive:{" "}
                        {v.archivePath
                          ? v.archiveAvailable
                            ? "present"
                            : "unavailable"
                          : "not created"}
                      </dd>
                    </dl>
                  </details>
                </article>
              ))}
            </div>
          )}
        </>
      )}
    </Modal>
  );
}
