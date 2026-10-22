import { Moon, ShieldCheck } from "lucide-react";
import type { Theme } from "../types";
import { version } from "../../package.json";
import { OpenAIConnection } from "./OpenAIConnection";
import { DesktopSettings } from "./DesktopSettings";
export function SettingsView({
  theme,
  busy,
  changeTheme,
}: {
  theme: Theme;
  busy: boolean;
  changeTheme: (theme: Theme) => void;
}) {
  return (
    <>
      <DesktopSettings />
      <OpenAIConnection />
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
            Discovery reads file metadata. Opt-in tracking captures immutable
            local snapshots. Separately enabled archival preserves older sources
            in Old Context Files.
          </p>
          <p>
            Registrations, history and preferences use per-user SQLite storage.
            Snapshot bytes remain in application data across upgrades and
            project removal. Monitoring operates only while ContextDock is
            running, including in the system tray. Use Exit in the tray menu to
            fully quit. Chats are also saved locally in the app database. Cloud
            sync enables OpenAI uploads and project questions; turning it off
            blocks new AI requests and keeps local history available. No
            telemetry.
          </p>
        </div>
      </section>
    </>
  );
}
