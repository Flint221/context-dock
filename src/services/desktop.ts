import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export interface DesktopStatus {
  startupEnabled?: boolean | null;
  keepRunning: boolean;
  startupRegistered: boolean;
  approvalNotifications: boolean;
  problemNotifications: boolean;
  uploadsPaused: boolean;
  pending: number;
  error: string | null;
}
export interface DesktopRoute {
  page: "review" | "project" | "settings";
  project: string | null;
}
export interface PendingFile {
  projectId: string;
  projectName: string;
  path: string;
  kind: string;
  size: number;
  status: string;
  modified: string | null;
}
export interface ReviewRequest {
  project: string;
  paths: string[];
  action: "approve" | "ignore";
}
export interface ReviewPlan extends ReviewRequest {
  token: string;
  projectName: string;
  revision: number;
  trackingEnabled: number;
  cloudEnabled: number;
  automatic: number;
}
const defaults: DesktopStatus = {
  keepRunning: true,
  startupRegistered: false,
  approvalNotifications: true,
  problemNotifications: true,
  uploadsPaused: false,
  pending: 0,
  error: null,
};
export const desktop = {
  onStopping: (handler: () => void) =>
    isTauri() ? listen("desktop-stopping", handler) : Promise.resolve(() => {}),
  native: () => isTauri(),
  read: () =>
    isTauri()
      ? invoke<DesktopStatus>("desktop_read")
      : Promise.resolve(defaults),
  setting: (key: string, value: boolean) =>
    invoke<void>("desktop_setting", { key, value }),
  subscribe: (handler: () => void) =>
    isTauri() ? listen("desktop-changed", handler) : Promise.resolve(() => {}),
  onRoute: async (handler: (route: DesktopRoute) => void) => {
    if (!isTauri()) return () => {};
    const stop = await listen<DesktopRoute>("desktop-route", (e) =>
      handler(e.payload),
    );
    try {
      const pending = await invoke<DesktopRoute | null>("desktop_route");
      if (pending) handler(pending);
      return stop;
    } catch (e) {
      stop();
      throw e;
    }
  },
  listenClose: async (handler: (action?: string) => void) => {
    if (!isTauri()) return () => {};
    const stop = await listen<string>("desktop-close", (e) =>
      handler(e.payload),
    );
    try {
      await invoke("desktop_ready");
      return stop;
    } catch (e) {
      stop();
      throw e;
    }
  },
  exit: async (action = "close") => {
    if (isTauri()) await invoke("desktop_complete", { action });
  },
  pending: () =>
    isTauri() ? invoke<PendingFile[]>("pending_read") : Promise.resolve([]),
  preview: (request: ReviewRequest) =>
    invoke<ReviewPlan>("pending_preview", { request }),
  apply: (request: ReviewRequest, token: string) =>
    invoke<void>("pending_apply", { request, token }),
};
