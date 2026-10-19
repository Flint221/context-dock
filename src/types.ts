export interface Project {
  id: string;
  name: string;
  path: string;
  createdAt: string;
  status: string;
  lastScan: string | null;
  scanError: string | null;
  documentCount: number;
  latestModified: string | null;
}
export interface Document {
  id: string;
  projectId: string;
  name: string;
  relativePath: string;
  absolutePath: string;
  kind: string;
  size: number;
  modifiedAt: string | null;
  lastObserved: string;
  status: string;
}
export type Theme = "system" | "light" | "dark";
export interface Snapshot {
  projects: Project[];
  documents: Document[];
  theme: Theme;
}
