import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { ProjectSummary } from "./ProjectSummary";
import type { CloudView } from "../cloud";
import type { TrackingView } from "../types";
const selection = {
  revision: 1,
  exclusions: [{ path: "private", kind: "folder" as const }],
  entries: [
    { path: "Notes_v6.md", kind: "file" as const, status: "available" },
    {
      path: "Private/Nested/Secret.md",
      kind: "file" as const,
      status: "available",
    },
  ],
  warnings: [],
  cloudPending: 0,
};
const tracking: TrackingView = {
  enabled: true,
  archiveEnabled: false,
  initialized: true,
  watchStatus: "watching",
  error: null,
  sources: [
    {
      id: "source",
      documentId: "doc",
      name: "Notes_v6.md",
      relativePath: "Notes_v6.md",
      status: "available",
      disposition: "tracked",
    },
  ],
  families: [
    {
      id: "doc",
      name: "Notes",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "source",
      activeFilename: "Notes_v6.md",
      sourceStatus: "available",
      currentNumber: 3,
      versionCount: 3,
      lastCaptured: null,
      needsReview: false,
    },
  ],
  reviews: [],
  activity: [],
  operations: [],
};
const cloud = {
  enabled: 1,
  paused: 0,
  connectionStatus: "connected",
  indexed: 8,
  pending: 1,
  failed: 0,
  families: [],
  status: "syncing",
} as unknown as CloudView;
it("counts saved selection and document families separately and colors indexing, failure and completion", () => {
  const { rerender } = render(
    <ProjectSummary
      selection={selection}
      tracking={tracking}
      cloud={cloud}
      unavailable={false}
    />,
  );
  expect(screen.getByText("Selected files").parentElement).toHaveTextContent(
    "Selected files1",
  );
  expect(screen.getByText("Tracked documents").parentElement).toHaveTextContent(
    "Tracked documents1",
  );
  expect(screen.getByText("8/9").parentElement).toHaveClass("summary-warning");
  rerender(
    <ProjectSummary
      selection={selection}
      tracking={tracking}
      cloud={{ ...cloud, pending: 0, failed: 1 }}
      unavailable={false}
    />,
  );
  expect(screen.getByText("8/9").parentElement).toHaveClass("summary-error");
  rerender(
    <ProjectSummary
      selection={selection}
      tracking={tracking}
      cloud={{ ...cloud, indexed: 9, pending: 0 }}
      unavailable={false}
    />,
  );
  expect(screen.getByText("9/9").parentElement).toHaveClass("summary-success");
  rerender(
    <ProjectSummary
      selection={selection}
      tracking={tracking}
      cloud={{ ...cloud, enabled: 0 }}
      unavailable={false}
    />,
  );
  expect(screen.getByText("Cloud off").parentElement).toHaveClass(
    "summary-neutral",
  );
});
it("never represents unavailable statuses as zero successful files or green indexing", () => {
  render(
    <ProjectSummary
      selection={null}
      tracking={null}
      cloud={null}
      unavailable={true}
    />,
  );
  expect(screen.getByText("Folder lost")).toBeInTheDocument();
  expect(screen.getByText("Cloud indexed").parentElement).toHaveClass(
    "summary-error",
  );
  expect(screen.getAllByText("—")).toHaveLength(3);
});

it("identifies failed archive operations separately from missing files and reviews", () => {
  render(<ProjectSummary selection={selection} tracking={{ ...tracking, operations: [
    { id: "a", kind: "archive", state: "blocked", error: "Unsupported filesystem", destination: "Old Context Files/Context_v3.md", createdAt: "" },
    { id: "b", kind: "archive", state: "blocked", error: "Unsupported filesystem", destination: "Old Context Files/Tasks_v2.md", createdAt: "" },
  ] }} cloud={cloud} unavailable={false} />);
  expect(screen.getByText("Needs attention").parentElement).toHaveTextContent("2 archive / file operations");
  expect(screen.getByText("archive: Old Context Files/Context_v3.md")).toBeInTheDocument();
  expect(screen.getByText("archive: Old Context Files/Tasks_v2.md")).toBeInTheDocument();
});

it("shows a finishing state instead of an error or ready status during verified removal", () => {
  render(<ProjectSummary selection={selection} tracking={tracking} unavailable={false}
    cloud={{...cloud, indexed: 12, pending: 0, failed: 0, error: null, status: "verifying_removal"}} />);
  expect(screen.getByText("Finishing sync").parentElement).toHaveClass("summary-warning");
  expect(screen.getByText("12/12").parentElement).toHaveClass("summary-warning");
});
