import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Snapshot, Theme } from "../types";
export const registry = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  add: (name: string, path: string) =>
    invoke<string>("add_project", { name, path }),
  rename: (id: string, name: string) =>
    invoke<void>("rename_project", { id, name }),
  remove: (id: string) => invoke<void>("remove_project", { id }),
  refresh: (id: string) => invoke<void>("refresh_project", { id }),
  open: (id: string) => invoke<void>("open_project", { id }),
  theme: (theme: Theme) => invoke<void>("set_theme", { theme }),
  pickFolder: () =>
    open({
      directory: true,
      multiple: false,
      title: "Choose a project folder",
    }),
};
