import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { drive, type DriveConnection } from "../drive";
import { beginOperation } from "../operations";

export function GoogleDriveConnection() {
  const [connection, setConnection] = useState<DriveConnection | null>(null);
  const [action, setAction] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const alive = useRef(true);
  const sequence = useRef(0);
  const refresh = useCallback(async () => {
    const request = ++sequence.current;
    const next = await drive.connection();
    if (alive.current && request === sequence.current) setConnection(next);
    return next;
  }, []);
  useEffect(() => {
    alive.current = true;
    void refresh().catch((e) => {
      if (alive.current) setError(String(e));
    });
    return () => {
      alive.current = false;
    };
  }, [refresh]);
  // A sign-in can continue while Settings is closed. Poll only during that
  // bounded interaction; disconnected/idle settings cause no recurring work.
  useEffect(() => {
    if (!connection?.connecting) return;
    const timer = window.setInterval(
      () => void refresh().catch(() => {}),
      1000,
    );
    return () => window.clearInterval(timer);
  }, [connection?.connecting, refresh]);
  async function run(
    name: string,
    task: () => Promise<void>,
    success: string,
    verify?: (c: DriveConnection) => boolean,
  ) {
    const release = beginOperation("google-drive-connection");
    if (!release) return;
    setAction(name);
    setError("");
    setMessage("");
    try {
      await task();
      const next = await refresh();
      if (verify && !verify(next))
        throw new Error(
          "Google Drive did not confirm the requested connection state. Refresh status and retry.",
        );
      if (alive.current) setMessage(success);
    } catch (e) {
      if (alive.current) setError(String(e));
      try {
        await refresh();
      } catch {
        /* Preserve the original operation error. */
      }
    } finally {
      release();
      if (alive.current) setAction("");
    }
  }
  const connecting =
    action === "connect" || action === "pick_root" || !!connection?.connecting;
  const busy = !!action || !!connection?.connecting;
  return (
    <section
      className="panel cloud-panel"
      aria-labelledby="drive-connection-heading"
    >
      <h2 id="drive-connection-heading">Google Drive connection</h2>
      <p>
        Publish the latest project documents to your own Google Drive. ChatGPT
        can read them through your connected Drive account.
      </p>
      <p>
        Status:{" "}
        <strong>
          {connecting
            ? "Waiting for Google sign-in"
            : connection?.connected
              ? "Connected"
              : connection?.configured
                ? "Ready to connect"
                : connection
                  ? "Setup needed"
                  : "Loading"}
        </strong>
        {connection?.account && <> · {connection.account.email}</>}
      </p>
      {!connection?.configured && (
        <p>
          Choose the Desktop OAuth client JSON for ContextDock, then sign in
          with Google. You only need to set up the client once.
        </p>
      )}
      <div className="cloud-actions">
        <button
          disabled={busy || !!connection?.connected}
          onClick={() =>
            void run(
              "import",
              async () => {
                const path = await open({
                  title: "Choose ContextDock Desktop OAuth client",
                  multiple: false,
                  directory: false,
                  filters: [
                    { name: "Google OAuth client", extensions: ["json"] },
                  ],
                });
                if (typeof path === "string")
                  await drive.action("import", path);
                else throw new Error("No OAuth client selected.");
              },
              "Desktop OAuth client saved. You can now sign in with Google.",
              (c) => c.configured,
            )
          }
        >
          {connection?.configured
            ? "Replace OAuth client"
            : "Choose OAuth client JSON"}
        </button>
        <button
          disabled={busy || !connection?.configured}
          onClick={() =>
            void run(
              "connect",
              () => drive.action("connect"),
              "Google Drive connected and account access verified.",
              (c) => c.connected && !!c.account,
            )
          }
        >
          {connection?.connected
            ? "Reconnect Google Drive"
            : "Sign in with Google"}
        </button>
        {connecting && (
          <button
            onClick={() =>
              void drive.cancel().catch((e) => setError(String(e)))
            }
          >
            Cancel sign-in
          </button>
        )}
        <button
          disabled={busy || !connection?.connected}
          onClick={() =>
            void run(
              "test",
              () => drive.action("test"),
              "Google Drive connection verified. No files were uploaded.",
              (c) => c.connected,
            )
          }
        >
          Test Drive connection
        </button>
        <button
          disabled={busy || !connection?.connected}
          onClick={() =>
            void run(
              "pick_root",
              () => drive.action("pick_root"),
              "Existing ContextDock folder authorized and saved.",
              (c) => c.connected,
            )
          }
        >
          Choose existing ContextDock folder
        </button>
        <button
          disabled={busy || !connection?.connected}
          onClick={() =>
            void run(
              "disconnect",
              () => drive.action("disconnect"),
              "Google Drive disconnected. Cloud files are kept.",
              (c) => !c.connected,
            )
          }
        >
          Disconnect Google Drive
        </button>
        <button
          disabled={busy}
          onClick={() =>
            void run(
              "refresh",
              async () => {
                await refresh();
              },
              "Connection status refreshed.",
            )
          }
        >
          Refresh Drive status
        </button>
      </div>
      {busy && (
        <p role="status">
          {connecting
            ? "Complete sign-in in your browser, then return here. You can cancel at any time."
            : "Updating and checking the Drive connection…"}
        </p>
      )}
      {message && <p role="status">{message}</p>}
      {(error || connection?.error) && (
        <p role="alert">{error || connection?.error}</p>
      )}
      <p>
        ContextDock requests access to files it creates or you explicitly
        authorize. Credentials stay in Windows Credential Manager. Disconnecting
        removes the saved sign-in and requests Google revocation; it keeps your
        cloud files.
      </p>
    </section>
  );
}
