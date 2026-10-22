import { useEffect, useState } from "react";
import { Bell, Monitor, Pause, Play } from "lucide-react";
import { desktop, type DesktopStatus } from "../services/desktop";
import { processes } from "../services/processes";
export function DesktopSettings() {
  const [status, setStatus] = useState<DesktopStatus | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [notificationTest, setNotificationTest] = useState("");
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    const load = () =>
      void desktop
        .read()
        .then((v) => {
          if (active) setStatus(v);
        })
        .catch((e) => {
          if (active) setError(String(e));
        });
    load();
    desktop
      .subscribe(load)
      .then((s) => {
        if (active) stop = s;
        else s();
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      stop?.();
    };
  }, []);
  async function change(key: string, value: boolean) {
    setBusy(true);
    setError("");
    try {
      await desktop.setting(key, value);
      setStatus(await desktop.read());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function testNotification() {
    setBusy(true);
    setError("");
    setNotificationTest("");
    try {
      setNotificationTest(await desktop.testNotification());
      setStatus(await desktop.read());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section
      className="panel settings desktop-settings"
      aria-label="Background and notifications"
    >
      <Monitor size={23} />
      <div>
        <h2>Background &amp; Windows startup</h2>
        <p>
          Keep approved documents current while ContextDock lives in your system
          tray. Exit from the tray menu stops background work.
        </p>
        {!desktop.native() && (
          <p>
            These controls are available in the installed Windows application.
          </p>
        )}
        {error && (
          <p role="alert" className="error-text">
            {error}
          </p>
        )}
        {status?.error && (
          <p role="alert" className="error-text">
            {status.error}
          </p>
        )}
        {status && (
          <>
            <label className="desktop-toggle">
              <input
                type="checkbox"
                checked={status.keepRunning}
                disabled={busy || !desktop.native()}
                onChange={(e) => void change("keepRunning", e.target.checked)}
              />
              Keep running when closed
            </label>
            <label className="desktop-toggle">
              <input
                type="checkbox"
                checked={status.startupRegistered}
                disabled={busy || !desktop.native()}
                onChange={(e) => void change("startup", e.target.checked)}
              />
              Start with Windows, hidden in the tray
            </label>
            <p>
              Startup registration is {status.startupRegistered ? "on" : "off"}.
              Windows can independently disable startup or suppress
              notifications.
            </p>
            {status.startupRegistered && status.startupEnabled === false && (
              <p className="error-text">
                Windows has disabled startup for ContextDock. Enable it in
                Windows startup settings to launch at sign-in.
              </p>
            )}
            <button
              disabled={!desktop.native()}
              onClick={() =>
                void processes.openStartup().catch((e) => setError(String(e)))
              }
            >
              Open Windows startup settings
            </button>
            <h3>
              <Bell size={18} /> Notifications
            </h3>
            <p>
              Grouped alerts appear only while ContextDock is hidden, minimized
              or unfocused. Document contents and filenames stay out of
              notifications.
            </p>
            <label className="desktop-toggle">
              <input
                type="checkbox"
                checked={status.approvalNotifications}
                disabled={busy || !desktop.native()}
                onChange={(e) =>
                  void change("approvalNotifications", e.target.checked)
                }
              />
              New files awaiting approval
            </label>
            <label className="desktop-toggle">
              <input
                type="checkbox"
                checked={status.problemNotifications}
                disabled={busy || !desktop.native()}
                onChange={(e) =>
                  void change("problemNotifications", e.target.checked)
                }
              />
              Tracking, folder and cloud problems
            </label>
            <button
              disabled={busy || !desktop.native()}
              onClick={() => void testNotification()}
            >
              <Bell size={16} /> Send test notification
            </button>
            <p>
              The test works even while this window is open. Click the alert to
              open Pending approvals. It does not upload or approve files. If a
              banner is hidden by Do not disturb, check Notification Center
              (Windows + N).
            </p>
            {notificationTest && <p role="status">{notificationTest}</p>}
            <button
              disabled={busy || !desktop.native()}
              onClick={() =>
                void change("uploadsPaused", !status.uploadsPaused)
              }
            >
              {status.uploadsPaused ? <Play size={16} /> : <Pause size={16} />}{" "}
              {status.uploadsPaused
                ? "Resume background uploads"
                : "Pause background uploads"}
            </button>
            <p>
              Pausing keeps local tracking running and retains cloud data.
              Existing indexing checks and confirmed removals continue. Each
              project’s saved sync settings still apply.
            </p>
          </>
        )}
      </div>
    </section>
  );
}
