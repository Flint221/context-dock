import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  Snapshot,
  Theme,
  TrackingView,
  Version,
  FolderPlan,
  PathRule,
  PathSelectionView,
  ArchivalReview,
} from "../types";
import type { Comparison } from "../comparison";
export const registry = {
  archivalReview: (project: string, enabled: boolean) =>
    invoke<ArchivalReview>("archival_review", { project, enabled }),
  applyArchival: (project: string, enabled: boolean, token: string) =>
    invoke<void>("apply_archival_review", { project, enabled, token }),
  paths: (project: string) =>
    invoke<PathSelectionView>("path_selection_view", { project }),
  savePaths: (project: string, revision: number, exclusions: PathRule[]) =>
    invoke<void>("save_path_selection", { project, revision, exclusions }),
  compare: (earlier: string, later: string) =>
    invoke<Comparison>("compare_versions", { earlier, later }),
  subscribe: (changed: () => void) =>
    listen<string>("registry-changed", changed),
  tracking: (project: string) =>
    invoke<TrackingView>("tracking_view", { project }),
  configureTracking: (
    project: string,
    enabled: boolean,
    archive: boolean,
    selected: string[] = [],
  ) =>
    invoke<void>("configure_tracking", { project, enabled, archive, selected }),
  resolve: (
    project: string,
    source: string,
    action: "assign" | "new" | "ignore",
    target: string | null = null,
  ) => invoke<void>("resolve_match", { project, source, action, target }),
  versions: (document: string) =>
    invoke<Version[]>("document_versions", { document }),
  openVersion: (version: string, archive: boolean) =>
    invoke<void>("open_version_location", { version, archive }),
  openCurrent: (document: string) =>
    invoke<void>("open_current_location", { document }),
  retry: (project: string) => invoke<void>("retry_operations", { project }),
  snapshot: () => invoke<Snapshot>("snapshot"),
  add: (name: string, path: string) =>
    invoke<string>("add_project", { name, path }),
  rename: (id: string, name: string) =>
    invoke<void>("rename_project", { id, name }),
  previewFolder: (id: string, path: string) =>
    invoke<FolderPlan>("preview_project_folder", { id, path }),
  reassignFolder: (id: string, path: string, token: string) =>
    invoke<void>("reassign_project_folder", { id, path, token }),
  remove: (id: string, retainCloud = false) =>
    invoke<void>("remove_project", { id, retainCloud }),
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
