import { CloudLink } from "./CloudLink";
import { useEffect, useRef, useState } from "react";
import { cloud, bytes, type Connection } from "../cloud";
export function OpenAIConnection() {
  const [connection, setConnection] = useState<Connection | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
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
  async function run(action: () => Promise<unknown>, success: string) {
    setBusy(true);
    setMessage("");
    try {
      await action();
      const next = await cloud.connection();
      if (alive.current) {
        setConnection(next);
        setMessage(success);
      }
    } catch (e) {
      if (alive.current) setMessage(String(e));
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  return (
    <section className="panel cloud-panel">
      <h2>OpenAI connection</h2>
      <p>
        Optional cloud indexing. Local discovery, snapshots and comparisons work
        without an API key. Each project needs a separate selection and upload
        consent.
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
          const request = cloud.save(input.value);
          input.value = "";
          void run(
            () => request,
            "Key saved. Test the connection before enabling a project.",
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
            Save / replace key
          </button>
          <button
            disabled={busy || !connection?.saved}
            type="button"
            onClick={() =>
              void run(
                () => cloud.action(null, "test"),
                "Connection tested with a read-only vector-store listing. No index or file was created.",
              )
            }
          >
            Test connection
          </button>
          <button
            disabled={busy || !connection?.saved}
            type="button"
            onClick={() =>
              void run(
                () => cloud.action(null, "disconnect_key"),
                "Saved key removed. Remote files and indexes remain in OpenAI.",
              )
            }
          >
            Disconnect OpenAI
          </button>
        </div>
      </form>
      {(message || connection?.error) && (
        <p role="status">{message || connection?.error}</p>
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
