import { invoke } from "@tauri-apps/api/core";
export const MINI = "gpt-5.4-mini-2026-03-17";
export const QUALITY = "gpt-5.4-2026-03-05";
export type Workflow = "compare" | "alignment" | "timeline" | "consistency";
export interface AnalysisRequest {
  kind: Workflow;
  versions: string[];
  baseline: string | null;
  model: string;
  inputBudget: number;
  outputTokens: number;
}
export interface Goals {
  purpose: string;
  goals: string;
  requirements: string;
  nonGoals: string;
  constraints: string;
  deliverables: string;
  principles: string;
}
export interface Source {
  current?: boolean;
  document: string;
  version: string;
  number: number;
  name: string;
  hash: string;
  capturedAt: string;
}
export interface Evidence {
  id: string;
  version: string;
  heading: string;
  start: number;
  end: number;
  text: string;
}
export interface Citation {
  evidenceId: string;
  quote: string;
}
export interface Finding {
  category: string;
  title: string;
  detail: string;
  atVersion: string | null;
  refs: Citation[];
}
export interface Result {
  summary: string;
  summaryRefs: Citation[];
  findings: Finding[];
}
export interface Baseline {
  id: string;
  title: string;
  revision: number;
  supersedes: string | null;
  source_version: string | null;
  source_hash: string | null;
  content: string;
  created_at: string;
  approved_at: string;
  active: number;
}
export interface Plan {
  project: string;
  request: AnalysisRequest;
  sources: Source[];
  baseline: (Baseline & { source?: Source }) | null;
  evidence: Evidence[];
  transitions: { earlier: string; later: string; blocks: unknown[] }[];
  warnings: string[];
  scope: string;
  promptVersion: number;
  schemaVersion: number;
  inputUnits: number;
  estimatedTokens: number;
  estimateUsd: number;
  conservativeUsd: number;
  pricingDate: string;
  cacheKey: string;
}
export interface Preview {
  id: string;
  plan: Plan;
  cached: string | null;
  requests: number;
  retrieval: boolean;
}
export interface Report {
  id: string;
  project_id: string;
  kind: Workflow;
  state: string;
  created_at: string;
  completed_at: string | null;
  error: string | null;
  plan: Plan;
  request: AnalysisRequest;
  result: Result | null;
  usage: { input_tokens?: number; output_tokens?: number } | null;
  response_id: string | null;
  decisions: { finding: number; decision: string; updated_at: string }[];
}
export interface Captured {
  id: string;
  document: string;
  number: number;
  capturedAt: string;
  name: string;
  current: number;
}
export interface IntelligenceView {
  projectName?: string;
  baselines: Baseline[];
  versions: Captured[];
  reports: Pick<
    Report,
    "id" | "kind" | "state" | "created_at" | "completed_at" | "error"
  >[];
}
export const ai = {
  action: <T>(project: string, action: string, payload: unknown = {}) =>
    invoke<T>("ai_action", { project, action, payload }),
};
