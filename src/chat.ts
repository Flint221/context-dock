import { invoke } from "@tauri-apps/api/core";
import type { Hit } from "./cloud";
export interface ChatSummary {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}
export interface ChatTurn {
  id: string;
  question: string;
  answer: string | null;
  state: "searching" | "running" | "completed" | "failed" | "uncertain";
  sources: (Hit & { citation: number })[];
  error: string | null;
  created_at: string;
}
export interface Chat extends ChatSummary {
  turns: ChatTurn[];
}
export const chat = {
  action: <T = void>(project: string, action: string, payload: unknown = {}) =>
    invoke<T>("chat_action", { project, action, payload }),
};
