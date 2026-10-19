import { FileText, Search } from "lucide-react";
import { useDeferredValue, useState } from "react";
import type { Document } from "../types";
import { date, size } from "../format";
export function Documents({ documents }: { documents: Document[] }) {
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const deferredQuery = useDeferredValue(query);
  const filtered = documents.filter((d) =>
    d.relativePath.toLowerCase().includes(deferredQuery.toLowerCase()),
  );
  return (
    <section className="panel">
      <div className="section-heading">
        <div>
          <h2>
            Documents <span className="count">{documents.length}</span>
          </h2>
          <p>Read-only metadata · Markdown and Word</p>
        </div>
        <label className="search">
          <Search size={16} />
          <input
            aria-label="Search documents"
            placeholder="Find a document…"
            value={query}
            onChange={(e) => (setQuery(e.target.value), setPage(0))}
          />
        </label>
      </div>
      {documents.length === 0 ? (
        <div className="empty">
          <FileText size={30} />
          <h3>No supported documents found</h3>
          <p>
            Add .md, .markdown, or .docx files to this folder, then refresh.
          </p>
        </div>
      ) : (
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Document</th>
                <th>Type</th>
                <th>Size</th>
                <th>Modified</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {filtered.slice(page * 100, (page + 1) * 100).map((d) => (
                <tr key={d.id}>
                  <td>
                    <div className="file-name">
                      <FileText size={17} />
                      <strong>{d.name}</strong>
                    </div>
                    <small title={d.absolutePath}>{d.relativePath}</small>
                  </td>
                  <td>{d.kind === "word" ? "Word" : "Markdown"}</td>
                  <td>{size(d.size)}</td>
                  <td>{date(d.modifiedAt)}</td>
                  <td>
                    <span className={`badge ${d.status}`}>{d.status}</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {filtered.length > 100 && (
            <div className="pagination">
              <button
                disabled={page === 0}
                onClick={() => setPage((p) => p - 1)}
              >
                Previous
              </button>
              <span>
                Page {page + 1} of {Math.ceil(filtered.length / 100)}
              </span>
              <button
                disabled={(page + 1) * 100 >= filtered.length}
                onClick={() => setPage((p) => p + 1)}
              >
                Next
              </button>
            </div>
          )}
          {filtered.length === 0 && (
            <p className="padded">No matching documents.</p>
          )}
        </div>
      )}
    </section>
  );
}
