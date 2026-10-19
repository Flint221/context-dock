export interface ComparedVersion {
  id: string;
  documentId: string;
  number: number;
  sourceName: string;
  capturedAt: string;
  sha256: string;
}
export interface DiffLine {
  kind: "equal" | "add" | "remove";
  oldLine: number | null;
  newLine: number | null;
  text: string;
  terminated: boolean;
}
export interface DiffBlock {
  id: string;
  kind: "equal" | "change";
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
  lines: DiffLine[];
}
export interface Comparison {
  formatVersion: number;
  documentId: string;
  documentName: string;
  kind: string;
  earlier: ComparedVersion;
  later: ComparedVersion;
  bytesIdentical: boolean;
  textEquivalent: boolean;
  addedLines: number;
  removedLines: number;
  changedBlocks: number;
  warnings: string[];
  blocks: DiffBlock[];
}
