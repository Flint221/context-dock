import { versionName } from "../format";
import { useCallback, useEffect, useRef, useState } from "react";
import { MessageSquare, Plus, Send, Trash2 } from "lucide-react";
import { chat, type Chat, type ChatSummary } from "../chat";
import { Modal } from "./Modal";
import { date } from "../format";
import { OperationStatus } from "./OperationStatus";
import { ChatAnswer } from "./ChatAnswer";

export function ProjectChat({
  project,
  enabled,
  ready,
  reason,
}: {
  project: string;
  enabled: boolean;
  ready: boolean;
  reason: string;
}) {
  const [chats, setChats] = useState<ChatSummary[]>([]);
  const [current, setCurrent] = useState<Chat | null>(null);
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [sending, setSending] = useState("");
  const [preview, setPreview] = useState("");
  const sendingNow = useRef(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [deleting, setDeleting] = useState(false);
  const alive = useRef(true);
  const request = useRef(0);
  const messages = useRef<HTMLDivElement>(null);
  const list = useCallback(async () => {
    const next = await chat.action<ChatSummary[]>(project, "list");
    if (alive.current) setChats(next);
    return next;
  }, [project]);
  useEffect(() => {
    alive.current = true;
    const token = ++request.current;
    void list()
      .then(async (items) => {
        if (!alive.current || token !== request.current || !items[0]) return;
        const saved = await chat.action<Chat>(project, "get", {
          id: items[0].id,
        });
        if (alive.current && token === request.current) setCurrent(saved);
      })
      .catch((e) => {
        if (alive.current) setError(String(e));
      })
      .finally(() => {
        if (alive.current) setLoading(false);
      });
    return () => {
      alive.current = false;
    };
  }, [list, project]);
  useEffect(() => {
    if (messages.current)
      messages.current.scrollTop = messages.current.scrollHeight;
  }, [current, busy, preview]);
  async function open(id: string) {
    const token = ++request.current;
    setError("");
    try {
      const saved = await chat.action<Chat>(project, "get", { id });
      if (alive.current && token === request.current) {
        setCurrent(saved);
        setQuestion("");
      }
    } catch (e) {
      if (alive.current) setError(String(e));
    }
  }
  async function send() {
    if (sendingNow.current || busy || !enabled || !ready || !question.trim())
      return;
    sendingNow.current = true;
    setBusy(true);
    setPreview("");
    setError("");
    const text = question.trim();
    setSending(text);
    let id = current?.id;
    try {
      if (!id) {
        const created = await chat.action<Chat>(project, "create");
        id = created.id;
        if (alive.current) setCurrent(created);
      }
      let accepting = true;
      let saved: Chat;
      try {
        saved = await chat.action<Chat>(
          project,
          "send",
          {
            id,
            question: text,
          },
          (answer) => {
            if (alive.current && accepting) setPreview(answer);
          },
        );
      } finally {
        accepting = false;
      }
      if (alive.current) {
        setCurrent(saved);
        setQuestion("");
      }
    } catch (e) {
      if (alive.current) {
        setError(String(e));
        if (id) {
          const saved = await chat
            .action<Chat>(project, "get", { id })
            .catch(() => null);
          if (saved && alive.current) setCurrent(saved);
        }
      }
    } finally {
      sendingNow.current = false;
      if (alive.current) {
        setPreview("");
        setBusy(false);
        setSending("");
        void list().catch(() => {});
      }
    }
  }
  return (
    <section className="panel project-chat" aria-label="Ask project">
      <div className="chat-heading">
        <div>
          <h2>
            <MessageSquare size={20} /> Ask project
          </h2>
          <p>
            Ask about your current synced documents. Chats are saved on this
            computer.
          </p>
        </div>
        <button
          disabled={busy || loading}
          onClick={() => {
            request.current++;
            setCurrent(null);
            setQuestion("");
            setError("");
          }}
        >
          <Plus size={16} /> New chat
        </button>
      </div>
      {!enabled && (
        <p className="chat-status">
          Cloud sync is off. Turn it on to ask questions. Your saved chats
          remain readable.
        </p>
      )}
      {enabled && !ready && (
        <p className="chat-status" role="status">
          {reason}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <div className="chat-layout">
        <nav className="chat-list" aria-label="Saved chats">
          {loading ? (
            <p>Loading chats…</p>
          ) : chats.length === 0 ? (
            <p>No saved chats yet.</p>
          ) : (
            chats.map((item) => (
              <button
                key={item.id}
                disabled={busy}
                aria-current={current?.id === item.id ? "page" : undefined}
                onClick={() => void open(item.id)}
              >
                {item.title}
              </button>
            ))
          )}
        </nav>
        <div className="chat-conversation">
          <div className="chat-title">
            <h3>{current?.title ?? "New chat"}</h3>
            {current && (
              <button
                className="danger"
                disabled={busy}
                onClick={() => setDeleting(true)}
              >
                <Trash2 size={15} /> Delete chat
              </button>
            )}
          </div>
          <div
            className="chat-messages"
            ref={messages}
            role="log"
            aria-label="Conversation"
            aria-live="polite"
          >
            {!current?.turns.length && (
              <p className="chat-empty">
                Try “What is this project building?” or “Which database does it
                use?”
              </p>
            )}
            {current?.turns.map((turn) => (
              <article className="chat-turn" key={turn.id}>
                <div className="chat-question">
                  <strong>You</strong>
                  <p>{turn.question}</p>
                </div>
                <div className="chat-answer">
                  <strong>ContextDock</strong>
                  {turn.answer && <ChatAnswer text={turn.answer} />}
                  {turn.error && <p role="alert">{turn.error}</p>}
                  {["running", "searching"].includes(turn.state) && (
                    <p>
                      Answer pending. Refresh this chat if the app was
                      interrupted.
                    </p>
                  )}
                  {turn.sources.length > 0 && (
                    <details>
                      <summary>Sources · {turn.sources.length}</summary>
                      {turn.sources.map((source) => (
                        <div
                          className="chat-source"
                          key={`${source.citation}:${source.versionId}`}
                        >
                          <strong>
                            [{source.citation}]{" "}
                            {source.sourceName ?? source.familyName} ·{" "}
                            {versionName(source.sourceName, source.number)}
                          </strong>
                        </div>
                      ))}
                    </details>
                  )}
                  <small>{date(turn.created_at)}</small>
                </div>
              </article>
            ))}
            {busy && sending && (
              <>
                <div className="chat-question">
                  <strong>You</strong>
                  <p>{sending}</p>
                </div>
                <div className="chat-answer" aria-busy="true">
                  <strong>ContextDock</strong>
                  <OperationStatus
                    busy
                    message={
                      preview ? "Writing answer…" : "Searching your documents…"
                    }
                  />
                  {preview && <ChatAnswer text={preview} />}
                </div>
              </>
            )}
          </div>
          <form
            className="chat-composer"
            onSubmit={(e) => {
              e.preventDefault();
              void send();
            }}
          >
            <label htmlFor={`question-${project}`}>Your question</label>
            <textarea
              id={`question-${project}`}
              placeholder={
                enabled
                  ? "Ask about this project…"
                  : "Turn on Cloud sync to ask a question"
              }
              rows={3}
              maxLength={2000}
              disabled={busy || !enabled || !ready}
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              onKeyDown={(e) => {
                if (
                  e.key === "Enter" &&
                  !e.shiftKey &&
                  !e.nativeEvent.isComposing
                ) {
                  e.preventDefault();
                  void send();
                }
              }}
            />
            <div className="chat-send-row">
              <small>
                Send uses OpenAI and may incur API charges. Answers use
                retrieved passages and up to six recent exchanges in this chat;
                check their sources. Enter to send; Shift+Enter for a new line.
              </small>
              <button
                className="primary"
                aria-busy={busy}
                disabled={busy || !enabled || !ready || !question.trim()}
                type="submit"
              >
                <Send size={16} /> {busy ? "Answering…" : "Send"}
              </button>
            </div>
          </form>
        </div>
      </div>
      {deleting && current && (
        <Modal
          title="Delete local chat?"
          close={() => {
            if (!busy) setDeleting(false);
          }}
        >
          <p>
            Delete “{current.title}” and its messages from this computer?
            Project documents, version history and cloud files are kept.
          </p>
          <div className="toolbar-actions">
            <button
              className="danger"
              disabled={busy}
              onClick={() => {
                setBusy(true);
                void chat
                  .action(project, "delete", { id: current.id })
                  .then(async () => {
                    if (alive.current) {
                      setCurrent(null);
                      setDeleting(false);
                    }
                    await list();
                  })
                  .catch((e) => {
                    if (alive.current) setError(String(e));
                  })
                  .finally(() => {
                    if (alive.current) setBusy(false);
                  });
              }}
            >
              Delete local chat
            </button>
            <button disabled={busy} onClick={() => setDeleting(false)}>
              Cancel
            </button>
          </div>
          {error && <p role="alert">{error}</p>}
        </Modal>
      )}
    </section>
  );
}
