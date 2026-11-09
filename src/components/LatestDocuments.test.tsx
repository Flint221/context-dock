import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { registry } from "../services/registry";
import { drive, type DriveProject } from "../drive";
import type { Project, TrackingView } from "../types";
import { LatestDocuments } from "./LatestDocuments";
vi.mock("../services/registry", () => ({
  registry: {
    tracking: vi.fn(),
    subscribe: vi.fn(),
    configureTracking: vi.fn(),
    resolve: vi.fn(),
    openCurrent: vi.fn(),
  },
}));
vi.mock("../drive", () => ({
  drive: { project: vi.fn(), subscribe: vi.fn() },
}));
const project: Project = {
  id: "p",
  name: "Project",
  path: "C:/Sample",
  createdAt: "",
  status: "available",
  lastScan: null,
  scanError: null,
  documentCount: 2,
  latestModified: null,
  trackingEnabled: true,
  archiveEnabled: false,
  trackingRevision: 0,
  folderState: "ready",
};
let tracking: TrackingView;
let publication: DriveProject;
beforeEach(() => {
  vi.resetAllMocks();
  tracking = {
    enabled: true,
    archiveEnabled: false,
    initialized: true,
    watchStatus: "watching",
    error: null,
    families: [
      {
        id: "doc",
        name: "Context",
        kind: "markdown",
        state: "tracked",
        activeSourceId: "v2",
        activeFilename: "ContextDock_Context_v2.md",
        sourceStatus: "available",
        currentNumber: 2,
        currentLabel: "v2",
        versionCount: 2,
        lastCaptured: null,
        needsReview: false,
      },
    ],
    sources: [
      {
        id: "v1",
        documentId: "doc",
        name: "ContextDock_Context_v1.md",
        relativePath: "ContextDock_Context_v1.md",
        status: "available",
        disposition: "superseded",
      },
      {
        id: "v2",
        documentId: "doc",
        name: "ContextDock_Context_v2.md",
        relativePath: "ContextDock_Context_v2.md",
        status: "available",
        disposition: "tracked",
      },
    ],
    reviews: [],
    activity: [],
    operations: [],
  };
  publication = {
    enabled: true,
    automatic: true,
    recycle: false,
    connected: true,
    accountId: "account",
    folderId: "folder",
    rootReady: true,
    error: null,
    documents: [
      {
        documentId: "doc",
        fileId: "stable-file",
        verifiedHash: "verified",
        verifiedName: "ContextDock_Context_v2.md",
        verifiedSize: 3,
        verifiedAt: "2026-11-09T00:00:00Z",
        pendingName: null,
        state: "synced",
        error: null,
      },
    ],
  };
  vi.mocked(registry.tracking).mockImplementation(async () =>
    structuredClone(tracking),
  );
  vi.mocked(drive.project).mockImplementation(async () =>
    structuredClone(publication),
  );
  vi.mocked(registry.subscribe).mockResolvedValue(() => {});
  vi.mocked(drive.subscribe).mockResolvedValue(() => {});
});
function page() {
  return render(
    <LatestDocuments
      project={project}
      changed={vi.fn(async () => {})}
      selection={null}
    />,
  );
}
it("shows one current row for v1 and v2 without history or archive controls", async () => {
  page();
  expect(await screen.findByText("Up to date")).toBeInTheDocument();
  const rows = within(screen.getByRole("table")).getAllByRole("row");
  expect(rows).toHaveLength(2);
  expect(rows[1]).toHaveTextContent("ContextDock_Context_v2.md");
  expect(
    screen.queryByText("ContextDock_Context_v1.md"),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: /history|archive|Ask AI/i }),
  ).not.toBeInTheDocument();
});
it.each(["auth", "attention"])(
  "shows a project %s block beside latest documents",
  async (blocked) => {
    publication.blocked = blocked;
    publication.error = "Review the Drive connection or destination.";
    page();
    expect(await screen.findByText("Needs attention")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Review the Drive connection or destination",
    );
    expect(screen.queryByText("Up to date")).not.toBeInTheDocument();
  },
);
it("shows destination setup as attention while keeping local monitoring available", async () => {
  publication.folderReady = false;
  page();
  expect(await screen.findByText("Needs attention")).toBeInTheDocument();
  expect(
    screen.getByRole("checkbox", { name: "Monitor selected files" }),
  ).toBeEnabled();
});
it("keeps a cleanup warning on the main file view without invalidating a verified cloud receipt", async () => {
  publication.cleanupError = "Original path is occupied; safety file kept.";
  page();
  expect(await screen.findByText("Up to date")).toBeInTheDocument();
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Open Drive publishing to retry recovery",
  );
});
it.each([
  ["uploading", "Uploading"],
  ["verifying", "Uploading"],
  ["waiting_file", "Waiting for file"],
  ["offline", "Offline"],
  ["needs_attention", "Needs attention"],
  ["queued", "Queued"],
])(
  "shows the pending %s state instead of a green receipt",
  async (state, label) => {
    publication.documents[0].state = state;
    page();
    expect(await screen.findByText(label)).toBeInTheDocument();
    expect(screen.queryByText("Up to date")).not.toBeInTheDocument();
  },
);
it("continues local tracking when Drive status cannot be read", async () => {
  vi.mocked(drive.project).mockRejectedValue(new Error("status unavailable"));
  page();
  expect(
    await screen.findByText(
      "Drive status is unavailable. Local file monitoring continues.",
    ),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("checkbox", { name: "Monitor selected files" }),
  ).toBeChecked();
  expect(screen.queryByText("Up to date")).not.toBeInTheDocument();
});
it("does not apply a family reassignment until its explicit action", async () => {
  tracking.reviews = [
    {
      sourceId: "candidate",
      name: "Candidate.md",
      relativePath: "Candidate.md",
      explanation: "Choose its family",
      candidateIds: ["doc"],
    },
  ];
  const user = userEvent.setup();
  page();
  await user.selectOptions(
    await screen.findByLabelText("Document family for Candidate.md"),
    "doc",
  );
  expect(registry.resolve).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Use this family" }));
  await waitFor(() =>
    expect(registry.resolve).toHaveBeenCalledWith(
      "p",
      "candidate",
      "assign",
      "doc",
    ),
  );
});
it("saves monitoring without enabling archival or sending a manual upload", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.configureTracking).mockImplementation(
    async (_id, enabled) => {
      tracking.enabled = enabled;
    },
  );
  page();
  const control = await screen.findByRole("checkbox", {
    name: "Monitor selected files",
  });
  await waitFor(() => expect(control).toBeChecked());
  await user.click(control);
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", false, false),
  );
});
