import { Moon, ShieldCheck } from "lucide-react";
import type { Theme } from "../types";
import { version } from "../../package.json";
import { DesktopSettings } from "./DesktopSettings";
import { GoogleDriveConnection } from "./GoogleDriveConnection";
export function SettingsView({
  theme,
  busy,
  changeTheme,
  openTools,
}: {
  theme: Theme;
  busy: boolean;
  changeTheme: (theme: Theme) => void;
  openTools?: () => void;
}) {
  return (
    <>
      <DesktopSettings />
      <GoogleDriveConnection />
      {openTools && (
        <details className="panel">
          <summary>Optional tools</summary>
          <button onClick={openTools}>Current processes</button>
        </details>
      )}
      <section className="panel settings">
        <Moon size={23} />
        <div>
          <h2>Appearance</h2>
          <p>Choose a theme for your workspace.</p>
          <p className="motion-preference">
            Animations follow your Windows reduced-motion preference. Progress
            and keyboard focus stay visible with motion turned off.
          </p>
          <label>
            Theme
            <select
              value={theme}
              disabled={busy}
              onChange={(e) => changeTheme(e.target.value as Theme)}
            >
              <option value="system">Use system setting</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </label>
        </div>
      </section>
      <section className="panel settings">
        <ShieldCheck size={23} />
        <div>
          <h2>About ContextDock</h2>
          <p>Version {version}</p>
          <p>
            Monitor selected project files, group filename versions and publish
            verified latest copies to your Google Drive when enabled.
          </p>
          <p>
            Registrations, file identities and preferences stay on this
            computer. Monitoring operates only while ContextDock is running,
            including in the system tray. Use Exit in the tray menu to fully
            quit. Existing archives and saved chats are preserved. New tracking
            does not create permanent historical copies. No telemetry.
          </p>
        </div>
      </section>
    </>
  );
}
