import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export interface Draft {
  enabled: boolean;
  automatic: boolean;
  mode: "current" | "history";
  documents: string[];
  softLimit: number;
}
export interface Plan {
  token: string;
  files: number;
  bytes: number;
  additions: number;
  removals: number;
  storeName: string;
}
export interface Family {
  id: string;
  name: string;
  eligible: number;
  selected: number;
  currentNumber: number | null;
  cloudNumber: number | null;
  versionCount: number;
  rawBytes: number;
}
export interface Job {
  id: string;
  version_id: string;
  file_id: string | null;
  familyName: string;
  sourceName?: string;
  number: number;
  state: string;
  error: string | null;
  usage_bytes: number | null;
  rawBytes: number;
}
export interface CloudView {
  selectionSaved?: boolean;
  selectionRemovalPending?: number;
  localReady?: boolean;
  localError?: string | null;
  connectionStatus: string;
  enabled: number;
  paused: number;
  auto_sync: number;
  started: number;
  mode: "current" | "history";
  soft_limit: number;
  store_id: string | null;
  store_name: string;
  store_state: string;
  status: string;
  error: string | null;
  created_at: string | null;
  last_sync: string | null;
  usage_bytes: number | null;
  families: Family[];
  jobs: Job[];
  activity: { kind: string; message: string; created_at: string }[];
  indexed: number;
  pending: number;
  failed: number;
  rawBytes: number;
  retainedFiles: number;
}
export interface Connection {
  saved: boolean;
  status: string;
  tested_at: string | null;
  error: string | null;
  usage: {
    project_id: string;
    name: string;
    store_id: string;
    status: string;
    usage_bytes: number | null;
    rawBytes: number;
  }[];
}
export interface Hit {
  fileId: string;
  familyName: string;
  sourceName?: string;
  number: number;
  versionId: string;
  score: number | null;
  excerpt: string;
}
export const cloud = {
  connection: () => invoke<Connection>("cloud_read", { project: null }),
  view: (project: string) => invoke<CloudView>("cloud_read", { project }),
  save: (key: string) => invoke<void>("cloud_key", { key }),
  action: <T = void>(
    project: string | null,
    action: string,
    payload: unknown = {},
  ) => invoke<T>("cloud_action", { project, action, payload }),
  subscribe: (changed: () => void) => listen("cloud-changed", changed),
};
export function bytes(value: number | null) {
  return value === null ? "Not yet measured" : `${value.toLocaleString()} B`;
}
