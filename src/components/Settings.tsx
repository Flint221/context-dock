import { Moon, ShieldCheck } from "lucide-react";
import type { Theme } from "../types";
import { version } from "../../package.json";
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
      <section className="panel settings">
        <Moon size={23} />
        <div>
          <h2>Appearance</h2>
          <p>Choose a theme for your workspace.</p>
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
          <p>Version {version} · Milestone 3</p>
          <p>
            Discovery reads file metadata. Opt-in tracking captures immutable
            local snapshots. Separately enabled archival preserves older sources
            in Old Context Files.
          </p>
          <p>
            Registrations, history and preferences use per-user SQLite storage.
            Snapshot bytes remain in application data across upgrades and
            project removal. Monitoring operates only while ContextDock is
            running. No telemetry or cloud connection.
          </p>
        </div>
      </section>
    </>
  );
}
