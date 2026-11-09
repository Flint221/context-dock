import { invoke } from "@tauri-apps/api/core";

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
    action: "import" | "connect" | "test" | "disconnect" | "forget_client",
    path: string | null = null,
  ) => invoke<void>("drive_action", { action, path }),
  cancel: () => invoke<void>("drive_cancel"),
};
