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
  trackingEnabled: boolean;
  archiveEnabled: boolean;
  trackingRevision: number;
  folderState: "ready" | "needs_reassignment";
}
export interface FolderPlan {
  path: string;
  previousPath: string;
  found: number;
  unchanged: number;
  changed: number;
  missing: string[];
  additional: number;
  token: string;
}
export interface Document {
  id: string;
  projectId: string;
  name: string;
  relativePath: string;
  absolutePath: string;
  documentId: string;
  kind: string;
  size: number;
  modifiedAt: string | null;
  lastObserved: string;
  status: string;
}
export type Theme = "system" | "light" | "dark";
export interface PathRule {
  path: string;
  kind: "folder" | "file";
}
export interface PathSelectionView {
  revision: number;
  exclusions: PathRule[];
  entries: { path: string; kind: "folder" | "file"; status: string }[];
  warnings: string[];
  cloudPending: number;
}
export interface Snapshot {
  projects: Project[];
  documents: Document[];
  theme: Theme;
}
export interface Family {
  id: string;
  name: string;
  kind: string;
  state: string;
  activeSourceId: string | null;
  activeFilename: string | null;
  sourceStatus: string | null;
  currentNumber: number | null;
  versionCount: number;
  lastCaptured: string | null;
  needsReview: boolean;
}
export interface SourceChoice {
  id: string;
  documentId: string;
  name: string;
  relativePath: string;
  status: string;
  disposition: string;
}
export interface PendingMatch {
  sourceId: string;
  name: string;
  relativePath: string;
  explanation: string;
  candidateIds: string[];
}
export interface Activity {
  id: string;
  kind: string;
  message: string;
  createdAt: string;
}
export interface PendingOperation {
  id: string;
  kind: string;
  state: string;
  error: string | null;
  createdAt: string;
}
export interface TrackingView {
  enabled: boolean;
  archiveEnabled: boolean;
  initialized: boolean;
  watchStatus: string;
  error: string | null;
  families: Family[];
  sources: SourceChoice[];
  reviews: PendingMatch[];
  activity: Activity[];
  operations: PendingOperation[];
}
export interface Version {
  id: string;
  number: number;
  sourceName: string;
  sourceRelative: string;
  capturedAt: string;
  modifiedAt: string | null;
  size: number;
  sha256: string;
  reason: string;
  snapshotPath: string;
  archivePath: string | null;
  snapshotAvailable: boolean;
  archiveAvailable: boolean;
  current: boolean;
}
