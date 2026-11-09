import { invoke } from "@tauri-apps/api/core";
import { subscribeChanges } from "./services/refresh";

export interface DrivePublication {
  documentId: string;
  fileId: string;
  verifiedHash: string | null;
  verifiedName: string | null;
  verifiedSize: number | null;
  verifiedAt: string | null;
  pendingName: string | null;
  state: string;
  error: string | null;
}
export interface DriveProject {
  enabled: boolean;
  automatic: boolean;
  recycle: boolean;
  connected: boolean;
  accountId: string | null;
  folderId: string | null;
  rootReady: boolean;
  folderReady?: boolean;
  error: string | null;
  documents: DrivePublication[];
  cleanupError?: string | null;
  blocked?: string | null;
}

export interface DriveConnection {
  configured: boolean;
  connected: boolean;
  connecting: boolean;
  account: { id: string; email: string; name: string } | null;
  error: string | null;
}
export const drive = {
  connection: () => invoke<DriveConnection>("drive_status"),
  action: (
    action:
      | "import"
      | "connect"
      | "test"
      | "disconnect"
      | "forget_client"
      | "pick_root",
    path: string | null = null,
  ) => invoke<void>("drive_action", { action, path, project: null }),
  cancel: () => invoke<void>("drive_cancel"),
  project: (project: string) =>
    invoke<DriveProject>("drive_project_read", { project }),
  configure: (project: string, enabled: boolean, automatic: boolean) =>
    invoke<void>("drive_project_action", {
      project,
      action: "configure",
      enabled,
      automatic,
    }),
  publish: (project: string) =>
    invoke<void>("drive_project_action", {
      project,
      action: "publish",
      enabled: null,
      automatic: null,
    }),
  subscribe: (project: string, changed: () => Promise<void>) =>
    subscribeChanges("drive-changed", changed, project),
  recycling: (project: string, recycle: boolean) =>
    invoke<void>("drive_project_action", {
      project,
      action: "recycle",
      recycle,
      enabled: null,
      automatic: null,
    }),
  chooseProjectFolder: (project: string) =>
    invoke<void>("drive_action", {
      action: "pick_project",
      path: null,
      project,
    }),
  recoverCleanup: (project: string) =>
    invoke<void>("drive_project_action", {
      project,
      action: "recover_cleanup",
      enabled: null,
      automatic: null,
      recycle: null,
    }),
  createFolders: (project: string) =>
    invoke<void>("drive_project_action", {
      project,
      action: "create_root",
      enabled: null,
      automatic: null,
    }),
};
