import { CloudLink } from "./CloudLink";
import { useEffect, useRef, useState } from "react";
import { cloud, bytes, type Connection } from "../cloud";
import { beginOperation } from "../operations";
import { OperationStatus } from "./OperationStatus";
import { Modal } from "./Modal";
export function OpenAIConnection() {
  const [connection, setConnection] = useState<Connection | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const key = useRef<HTMLInputElement>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    void cloud
      .connection()
      .then((c) => {
        if (alive.current) setConnection(c);
      })
      .catch((e) => {
        if (alive.current) setMessage(String(e));
      });
    return () => {
      alive.current = false;
    };
  }, []);
  async function run(
    action: () => Promise<unknown>,
    success: string,
    verify?: (value: Connection) => boolean,
  ) {
    const release = beginOperation("openai-connection");
    if (!release) return false;
    setBusy(true);
    setMessage("");
    setFailed(false);
    let complete = false;
    try {
      await action();
      const next = await cloud.connection();
      if (alive.current) setConnection(next);
      if (verify && !verify(next))
        throw new Error(
          "The backend did not confirm the requested OpenAI connection state. Refresh or test the connection before trying again.",
        );
      if (alive.current) {
        setMessage(success);
      }
      complete = true;
    } catch (e) {
      if (alive.current) {
        setMessage(String(e));
        setFailed(true);
      }
    } finally {
      // Saving may succeed even when the subsequent read-only key test fails.
      // Refresh that state so Test connection remains available for recovery.
      if (!complete) {
        try {
          const next = await cloud.connection();
          if (alive.current) setConnection(next);
        } catch (e) {
          if (alive.current) setMessage((previous) => previous || String(e));
        }
      }
      release();
      if (alive.current) setBusy(false);
    }
    return complete;
  }
  return (
    <section className="panel cloud-panel">
      <h2>OpenAI connection</h2>
      <p>
        <CloudLink href="https://platform.openai.com/api-keys">
          Create or manage your personal OpenAI API key
        </CloudLink>
        . Enter it only here; do not share it in chat.
      </p>
      <p>
        Local version history, comparisons and saved chats work without an API
        key. Turn on Cloud sync in a project to upload selected documents and
        ask questions. Each Send is an explicit, potentially paid AI request.
      </p>
      <p>
        Status: <strong>{connection?.status ?? "Loading"}</strong> · Key{" "}
        {connection?.saved
          ? "saved in Windows Credential Manager"
          : "not saved"}
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const input = key.current!;
          void run(
            async () => {
              const value = input.value;
              input.value = "";
              await cloud.save(value);
              await cloud.action(null, "test");
            },
            "Key saved and connection verified. Turn on Cloud sync in your project to ask questions.",
            (next) => next.saved && next.status === "connected",
          );
        }}
      >
        <label>
          API key
          <input
            ref={key}
            type="password"
            autoComplete="off"
            spellCheck={false}
            required
            disabled={busy}
            placeholder="Paste your OpenAI API key"
          />
        </label>
        <div className="cloud-actions">
          <button disabled={busy} type="submit">
            Save and test key
          </button>
          <button
            disabled={busy || !connection?.saved}
            type="button"
            onClick={() =>
              void run(
                () => cloud.action(null, "test"),
                "Connection tested with a read-only vector-store listing. No index or file was created.",
                (next) => next.status === "connected",
              )
            }
          >
            Test connection
          </button>
          <button
            disabled={busy || !connection?.saved}
            type="button"
            onClick={() => {
              setMessage("");
              setFailed(false);
              setDisconnecting(true);
            }}
          >
            Disconnect OpenAI
          </button>
        </div>
      </form>
      <OperationStatus
        busy={busy}
        message="Applying and verifying the OpenAI connection…"
        success={!failed ? message : undefined}
      />
      {failed && message && <p role="alert">{message}</p>}
      {!message && connection?.error && <p role="alert">{connection.error}</p>}
      {disconnecting && (
        <Modal
          title="Disconnect OpenAI?"
          close={() => {
            if (!busy) setDisconnecting(false);
          }}
        >
          <p>
            Remove the saved key and stop new OpenAI requests for all projects.
            Local history and chats are kept. First turn off Cloud sync and
            finish cloud deletion for active projects. Data explicitly retained
            when removing a project remains online until you delete it
            separately.
          </p>
          <OperationStatus
            busy={busy}
            message="Disconnecting and verifying key removal…"
          />
          {failed && message && <p role="alert">{message}</p>}
          <div className="cloud-actions">
            <button
              className="danger"
              disabled={busy}
              onClick={() => {
                void run(
                  () => cloud.action(null, "disconnect_key"),
                  "OpenAI disconnected — confirmed. Saved key removed. Any cloud data explicitly retained when removing projects remains in OpenAI.",
                  (next) => !next.saved && next.status === "disconnected",
                ).then((ok) => {
                  if (ok && alive.current) setDisconnecting(false);
                });
              }}
            >
              {busy ? "Disconnecting…" : "Disconnect OpenAI"}
            </button>
            <button disabled={busy} onClick={() => setDisconnecting(false)}>
              Cancel
            </button>
          </div>
        </Modal>
      )}
      <p>
        The key is never saved in SQLite or browser storage. Replacing it
        requires testing and verifying access to each existing project index.
        Test success confirms listing access, not billing or upload permissions.
      </p>
      <h3>Cloud usage</h3>
      <p>
        Raw snapshot size differs from OpenAI's parsed/indexed storage. Usage
        below is the last measured value, not a spending cap. Your API billing
        is separate from ChatGPT. Upload and storage limits or charges may
        apply; indexing is not guaranteed to be free.
      </p>
      <div className="cloud-actions">
        <CloudLink href="https://platform.openai.com/usage">
          OpenAI usage
        </CloudLink>
        <CloudLink href="https://platform.openai.com/settings/organization/billing/overview">
          Billing
        </CloudLink>
        <CloudLink href="https://developers.openai.com/api/docs/pricing">
          Current pricing
        </CloudLink>
      </div>
      {connection?.usage.map((p) => (
        <p key={p.project_id}>
          <strong>{p.name}</strong> · {p.status} · raw {bytes(p.rawBytes)} ·
          indexed {bytes(p.usage_bytes)} <code>{p.store_id}</code>
        </p>
      ))}
    </section>
  );
}
