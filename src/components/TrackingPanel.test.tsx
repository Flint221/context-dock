import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, it, expect, vi } from "vitest";
import { TrackingPanel } from "./TrackingPanel";
import { registry } from "../services/registry";
import type { Project, TrackingView, Version } from "../types";
vi.mock("../services/registry", () => ({
  registry: {
    tracking: vi.fn(),
    configureTracking: vi.fn(),
    archivalReview: vi.fn(),
    applyArchival: vi.fn(),
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
  folderState: "ready",
};
let view: TrackingView;
const changed = vi.fn(async () => {});
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(registry.archivalReview).mockImplementation(
    async (_project, enabled) => ({
      token: "review-token",
      enabled,
      items: [],
      warnings: [],
    }),
  );
  vi.mocked(registry.applyArchival).mockImplementation(
    async (project, enabled) =>
      registry.configureTracking(project, view.enabled, enabled),
  );
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
it("initializes folder-selected sources with archival off by default", async () => {
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("button", { name: "Enable tracking…" }),
  );
  const dialog = within(screen.getByRole("dialog"));
  expect(
    dialog.getByText(/one document family, imported in numeric order/),
  ).toBeInTheDocument();
  expect(
    dialog.getByRole("checkbox", { name: /Also enable automatic archival/ }),
  ).not.toBeChecked();
  expect(dialog.getByText("Technical_Documentation.md")).toBeInTheDocument();
  expect(
    dialog.queryByRole("checkbox", { name: "Technical_Documentation.md" }),
  ).not.toBeInTheDocument();
  await user.click(dialog.getByText("Capture selected and enable"));
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", true, false, [
      "s1",
      "s2",
    ]),
  );
  await screen.findByRole("button", { name: "Disable tracking" });
});

it("reviews existing archive paths without moving files and allows cancelling a failed review", async () => {
  view.enabled = true;
  view.initialized = true;
  view.archiveEnabled = true;
  vi.mocked(registry.archivalReview).mockRejectedValueOnce(
    new Error("Folder is unavailable"),
  );
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("button", { name: "Organize older files now" }),
  );
  const dialog = within(screen.getByRole("dialog"));
  expect(await dialog.findByRole("alert")).toHaveTextContent(
    "Folder is unavailable",
  );
  expect(dialog.getByRole("button", { name: "Organize files" })).toBeDisabled();
  vi.mocked(registry.archivalReview).mockResolvedValueOnce({
    token: "paths",
    enabled: true,
    items: [
      {
        from: "Report_v1.md",
        to: "Old Context Files/Report/Report_v1.md",
        action: "archive",
        problem: null,
      },
    ],
    warnings: [],
  });
  await user.click(dialog.getByRole("button", { name: "Reload review" }));
  expect(await dialog.findByText(/Archive: Report_v1.md/)).toBeInTheDocument();
  expect(dialog.getByText(/To: Old Context Files/)).toBeInTheDocument();
  expect(registry.applyArchival).not.toHaveBeenCalled();
  await user.click(dialog.getByRole("button", { name: "Cancel" }));
  expect(
    screen.getByRole("checkbox", { name: /Enable automatic archival/ }),
  ).toBeChecked();
});

it("keeps a stale archival review open, reloads it, and applies only the new token", async () => {
  view.enabled = true;
  view.initialized = true;
  vi.mocked(registry.applyArchival).mockRejectedValueOnce(
    new Error("The archival review changed. Reload the review."),
  );
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("checkbox", { name: /Enable automatic archival/ }),
  );
  const dialog = within(screen.getByRole("dialog"));
  await waitFor(() =>
    expect(
      dialog.getByRole("button", { name: "Enable archival" }),
    ).toBeEnabled(),
  );
  await user.click(dialog.getByRole("button", { name: "Enable archival" }));
  expect(await dialog.findByRole("alert")).toHaveTextContent("review changed");
  vi.mocked(registry.archivalReview).mockResolvedValueOnce({
    token: "new-review",
    enabled: true,
    items: [],
    warnings: [],
  });
  await user.click(dialog.getByRole("button", { name: "Reload review" }));
  await waitFor(() =>
    expect(
      dialog.getByRole("button", { name: "Enable archival" }),
    ).toBeEnabled(),
  );
  await user.click(dialog.getByRole("button", { name: "Enable archival" }));
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(registry.applyArchival).toHaveBeenLastCalledWith(
    "p",
    true,
    "new-review",
  );
  expect(
    screen.getByRole("checkbox", { name: /Enable automatic archival/ }),
  ).toBeChecked();
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
  expect(registry.configureTracking).not.toHaveBeenCalled();
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Enable archival",
    }),
  );
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", true, true),
  );
  await user.click(
    await screen.findByRole("button", { name: "Disable tracking" }),
  );
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Turn off tracking",
    }),
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
it("keeps prior separate histories accessible without counting them as active families", async () => {
  view.enabled = true;
  view.initialized = true;
  const family = {
    id: "combined",
    name: "Report",
    kind: "markdown",
    state: "tracked",
    activeSourceId: "s2",
    activeFilename: "Report_v2.md",
    sourceStatus: "available",
    currentNumber: 2,
    versionCount: 2,
    lastCaptured: null,
    needsReview: false,
  };
  view.families = [
    family,
    {
      ...family,
      id: "original",
      state: "grouped_history",
      activeSourceId: null,
      activeFilename: null,
      currentNumber: 1,
      versionCount: 1,
    },
  ];
  vi.mocked(registry.versions).mockResolvedValue([]);
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  expect(await screen.findByText("Report_v2.md")).toBeInTheDocument();
  expect(screen.getAllByRole("button", { name: "View history" })).toHaveLength(
    1,
  );
  await user.click(screen.getByText("Earlier separate histories (1)"));
  await user.click(
    screen.getByRole("button", { name: "View earlier history of Report" }),
  );
  await waitFor(() =>
    expect(registry.versions).toHaveBeenCalledWith("original"),
  );
});
it("shows the filename version as Current separately from the local snapshot count", async () => {
  view.enabled = true;
  view.initialized = true;
  view.families = [
    {
      id: "master",
      name: "RAP Master Context",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "s1",
      activeFilename: "RAP_Master_Context_v6.md",
      sourceStatus: "available",
      currentNumber: 3,
      currentLabel: "v6",
      versionCount: 3,
      lastCaptured: null,
      needsReview: false,
    },
    {
      id: "readme",
      name: "Readme",
      kind: "markdown",
      state: "tracked",
      activeSourceId: "s2",
      activeFilename: "Readme.md",
      sourceStatus: "available",
      currentNumber: 2,
      currentLabel: null,
      versionCount: 2,
      lastCaptured: null,
      needsReview: false,
    },
  ];
  render(<TrackingPanel project={project} changed={changed} />);
  const row = await screen.findByRole("row", { name: /RAP Master Context/ });
  expect(within(row).getByRole("cell", { name: "v6" })).toHaveAttribute(
    "title",
    "Current saved capture #3",
  );
  expect(within(row).getByRole("cell", { name: "3" })).toBeInTheDocument();
  const unversioned = screen.getByRole("row", { name: /Readme/ });
  expect(
    within(unversioned).getByRole("cell", { name: "Working copy" }),
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
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Resume tracking",
    }),
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
it("cancels archival changes and refuses success when the saved flags disagree", async () => {
  view.enabled = true;
  view.initialized = true;
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  const archival = await screen.findByRole("checkbox", {
    name: /Enable automatic archival of/,
  });
  await user.click(archival);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }),
  );
  expect(archival).not.toBeChecked();
  expect(registry.configureTracking).not.toHaveBeenCalled();
  vi.mocked(registry.configureTracking).mockResolvedValueOnce(undefined);
  await user.click(archival);
  const dialog = screen.getByRole("dialog");
  await user.click(
    within(dialog).getByRole("button", { name: "Enable archival" }),
  );
  expect(await within(dialog).findByRole("alert")).toHaveTextContent(
    "backend did not confirm",
  );
  expect(archival).not.toBeChecked();
  expect(dialog).toBeInTheDocument();
  expect(
    within(dialog).getByRole("button", { name: "Enable archival" }),
  ).toBeEnabled();
});

it("round-trips the archival checkbox with cancellation and verified off state while tracking stays on", async () => {
  view.enabled = true;
  view.initialized = true;
  vi.mocked(registry.configureTracking).mockImplementation(
    async (_p, enabled, archive) => {
      view.enabled = enabled;
      view.archiveEnabled = archive;
    },
  );
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  const checkbox = await screen.findByRole("checkbox", {
    name: /Enable automatic archival of/,
  });
  await user.click(checkbox);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Enable archival",
    }),
  );
  await waitFor(() => expect(checkbox).toBeChecked());
  await user.click(checkbox);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }),
  );
  expect(checkbox).toBeChecked();
  expect(registry.configureTracking).toHaveBeenCalledTimes(1);
  await user.click(checkbox);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Turn off archival",
    }),
  );
  await waitFor(() => expect(checkbox).not.toBeChecked());
  expect(view.enabled).toBe(true);
  expect(registry.configureTracking).toHaveBeenLastCalledWith(
    project.id,
    true,
    false,
  );
});
it("sends a tracking toggle once and keeps its spinner until state verification finishes", async () => {
  view.enabled = true;
  view.initialized = true;
  const user = userEvent.setup();
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  vi.mocked(registry.configureTracking).mockImplementation(
    async (_p, enabled, archive) => {
      await pending;
      view.enabled = enabled;
      view.archiveEnabled = archive;
    },
  );
  render(<TrackingPanel project={project} changed={changed} />);
  await user.click(
    await screen.findByRole("button", { name: "Disable tracking" }),
  );
  const dialog = screen.getByRole("dialog");
  const confirm = within(dialog).getByRole("button", {
    name: "Turn off tracking",
  });
  act(() => {
    fireEvent.click(confirm);
    fireEvent.click(confirm);
  });
  expect(registry.configureTracking).toHaveBeenCalledTimes(1);
  expect(confirm).toBeDisabled();
  expect(within(dialog).getByRole("status")).toHaveTextContent(
    "checking the backend",
  );
  await act(async () => finish());
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(screen.getByRole("button", { name: "Resume tracking" })).toBeEnabled();
  expect(screen.getByRole("status")).toHaveTextContent("saved and confirmed");
});

it("restores archived sources while keeping tracking paused", async () => {
  view.initialized = true;
  view.enabled = false;
  view.archiveEnabled = true;
  const user = userEvent.setup();
  render(<TrackingPanel project={project} changed={changed} />);
  const checkbox = await screen.findByRole("checkbox", {
    name: /Enable automatic archival/,
  });
  expect(checkbox).toBeEnabled();
  await user.click(checkbox);
  const dialog = within(screen.getByRole("dialog"));
  await user.click(
    await dialog.findByRole("button", { name: "Turn off archival" }),
  );
  await waitFor(() =>
    expect(registry.configureTracking).toHaveBeenCalledWith("p", false, false),
  );
  expect(
    await screen.findByRole("button", { name: "Resume tracking" }),
  ).toBeInTheDocument();
  expect(
    await screen.findByRole("button", { name: "Restore moved files…" }),
  ).toBeEnabled();
});
