import { invoke } from "@tauri-apps/api/core";

export interface Endpoint {
  protocol: "TCP" | "UDP";
  address: string;
  port: number;
  scope: number;
}
export interface LocalProcess {
  pid: number;
  parentPid: number;
  name: string;
  path: string;
  started: string;
  startedAt: string | null;
  canStop: boolean;
  protection: string;
  endpoints: Endpoint[];
}
export interface ProcessInventory {
  processes: LocalProcess[];
  warnings: string[];
  scannedAt: string;
}
export interface StopResult {
  pid: number;
  stopped: boolean;
  message: string;
}
export interface StartupItem {
  name: string;
  source: string;
}
export const processKey = (p: LocalProcess) => `${p.pid}:${p.started}`;
export const processTarget = (p: LocalProcess) => ({
  pid: p.pid,
  started: p.started,
});
export const isLocalEndpoint = (e: Endpoint) =>
  e.address === "0.0.0.0" ||
  e.address === "::" ||
  e.address === "::1" ||
  /^127\./.test(e.address);
export const endpointLabel = (e: Endpoint) =>
  `${e.protocol} ${e.address.includes(":") ? `[${e.address}${e.scope ? `%${e.scope}` : ""}]` : e.address}:${e.port}`;
export const processes = {
  inventory: () => invoke<ProcessInventory>("process_inventory"),
  stop: (items: LocalProcess[]) =>
    invoke<StopResult[]>("stop_processes", {
      targets: items.map(processTarget),
    }),
  preview: (p: LocalProcess, endpoint: Endpoint, secure: boolean) =>
    invoke<void>("preview_process", {
      target: processTarget(p),
      endpoint,
      secure,
    }),
  startup: () => invoke<StartupItem[]>("process_startup_items"),
  openStartup: () => invoke<void>("open_startup_settings"),
};
