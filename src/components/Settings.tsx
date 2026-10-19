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
          <p>Version {version} · Milestone 1</p>
          <p>
            A local project and document registry. Scanning reads file metadata;
            it does not index contents or create versions.
          </p>
          <p>
            Your registrations and preferences are stored in a per-user SQLite
            database. No telemetry or cloud connection.
          </p>
        </div>
      </section>
    </>
  );
}
