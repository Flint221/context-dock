import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, it, expect, vi } from "vitest";
import { TrackingPanel } from "./TrackingPanel";
import { registry } from "../services/registry";
import type { Project, TrackingView, Version } from "../types";
vi.mock("../services/registry", () => ({
  registry: {
    tracking: vi.fn(),
    configureTracking: vi.fn(),
    resolve: vi.fn(),
    versions: vi.fn(),
    openVersion: vi.fn(),
    openCurrent: vi.fn(),
    refresh: vi.fn(),
    retry: vi.fn(),
  },
}));
const project: Project = {
  id: "p",
  name: "Test",
  path: "C:\\Test",
  createdAt: "",
  status: "available",
  lastScan: null,
  scanError: null,
  documentCount: 2,
  latestModified: null,
  trackingEnabled: false,
  archiveEnabled: false,
  trackingRevision: 0,
};
let view: TrackingView;
const changed = vi.fn(async () => {});
beforeEach(() => {
  vi.clearAllMocks();
  view = {
    enabled: false,
    archiveEnabled: false,
    initialized: false,
    watchStatus: "off",
    error: null,
    families: [],
    sources: [
      {
        id: "s1",
        documentId: "d1",
        name: "Master_Context_v1.md",
        relativePath: "Master_Context_v1.md",
        status: "available",
        disposition: "discovered",
      },
      {
        id: "s2",
        documentId: "d2",
        name: "Technical_Documentation.md",
        relativePath: "Technical_Documentation.md",
        status: "available",
        disposition: "discovered",
      },
    ],
    reviews: [],
    activity: [],
    operations: [],
  };
  vi.mocked(registry.tracking).mockImplementation(async () =>
    structuredClone(view),
  );
  vi.mocked(registry.configureTracking).mockImplementation(
    async (_p, enabled, archive) => {
      view.enabled = enabled;
      view.archiveEnabled = archive;
      view.initialized = true;
    },
  );
});
it("initializes only selected sources with archival off by default", async () => {
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("button", { name: "Enable tracking…" }),
  );
  const dialog = within(screen.getByRole("dialog"));
  expect(
    dialog.getByRole("checkbox", { name: /Also enable automatic archival/ }),
  ).not.toBeChecked();
  await user.click(
    dialog.getByRole("checkbox", { name: "Technical_Documentation.md" }),
  );
  await user.click(dialog.getByText("Capture selected and enable"));
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", true, false, [
      "s1",
    ]),
  );
  await screen.findByRole("button", { name: "Disable tracking" });
});
it("requires a separate archival toggle and retains history controls when tracking is paused", async () => {
  view.enabled = true;
  view.initialized = true;
  view.watchStatus = "watching";
  view.families = [
    {
      id: "d1",
      name: "Master Context",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "s1",
      activeFilename: "Master_Context_v1.md",
      sourceStatus: "available",
      currentNumber: 2,
      versionCount: 2,
      lastCaptured: "2026-10-19T12:00:00Z",
      needsReview: false,
    },
  ];
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("checkbox", {
      name: /Enable automatic archival of/,
    }),
  );
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", true, true),
  );
  await user.click(
    await screen.findByRole("button", { name: "Disable tracking" }),
  );
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", false, true),
  );
  expect(
    await screen.findByRole("button", { name: "Resume tracking" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "View history" }),
  ).toBeInTheDocument();
});
it("holds ambiguous files for an explicit family selection", async () => {
  view.enabled = true;
  view.initialized = true;
  view.reviews = [
    {
      sourceId: "s2",
      name: "New.md",
      relativePath: "New.md",
      candidateIds: ["d1"],
      explanation: "Two document families could match. No sources moved.",
    },
  ];
  view.families = [
    {
      id: "d1",
      name: "Master Context",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "s1",
      activeFilename: "Master_Context_v1.md",
      sourceStatus: "available",
      currentNumber: 1,
      versionCount: 1,
      lastCaptured: null,
      needsReview: false,
    },
  ];
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  const select = await screen.findByLabelText("Assign to document family");
  expect(
    screen.getByRole("button", { name: "Assign as version" }),
  ).toBeDisabled();
  expect(registry.resolve).not.toHaveBeenCalled();
  await user.selectOptions(select, "d1");
  await user.click(screen.getByRole("button", { name: "Assign as version" }));
  await waitFor(() =>
    expect(registry.resolve).toHaveBeenCalledWith("p", "s2", "assign", "d1"),
  );
});
it("opens a preserved snapshot location by version ID and exposes its original metadata", async () => {
  view.enabled = true;
  view.initialized = true;
  view.families = [
    {
      id: "d1",
      name: "Master Context",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "s1",
      activeFilename: "Master_Context_v1.md",
      sourceStatus: "available",
      currentNumber: 1,
      versionCount: 1,
      lastCaptured: null,
      needsReview: false,
    },
  ];
  const version: Version = {
    id: "v1",
    number: 1,
    sourceName: "Master_Context_v1.md",
    sourceRelative: "Master_Context_v1.md",
    capturedAt: "2026-10-19T12:00:00Z",
    modifiedAt: null,
    size: 10,
    sha256: "a".repeat(64),
    reason: "initial_tracking",
    snapshotPath: "C:\\AppData\\snapshots\\v1.md",
    archivePath: null,
    snapshotAvailable: true,
    archiveAvailable: false,
    current: true,
  };
  vi.mocked(registry.versions).mockResolvedValue([version]);
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(await screen.findByRole("button", { name: "View history" }));
  await user.click(
    await screen.findByRole("button", { name: "Snapshot location" }),
  );
  expect(registry.openVersion).toHaveBeenCalledWith("v1", false);
  await user.click(screen.getByText("Version metadata"));
  expect(screen.getByText("a".repeat(64))).toBeInTheDocument();
});

it("unlocks resume decisions and scan controls after success or failure", async () => {
  view.initialized = true;
  view.reviews = [
    {
      sourceId: "s2",
      name: "New.md",
      relativePath: "New.md",
      candidateIds: [],
      explanation: "Select a family or keep independent.",
    },
  ];
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("button", { name: "Resume tracking" }),
  );
  await waitFor(() =>
    expect(screen.getByLabelText("Assign to document family")).toBeEnabled(),
  );
  expect(
    screen.getByRole("button", { name: "Disable tracking" }),
  ).toBeEnabled();
  vi.mocked(registry.refresh).mockRejectedValueOnce(
    "Source is locked; retry after closing the writer.",
  );
  await user.click(screen.getByRole("button", { name: "Scan for changes" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Source is locked",
  );
  expect(
    screen.getByRole("button", { name: "Scan for changes" }),
  ).toBeEnabled();
  expect(screen.getByLabelText("Assign to document family")).toBeEnabled();
  vi.mocked(registry.refresh).mockResolvedValueOnce(undefined);
  await user.click(screen.getByRole("button", { name: "Scan for changes" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Scan for changes" }),
    ).toBeEnabled(),
  );
  expect(screen.queryByText("Working…")).not.toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});
