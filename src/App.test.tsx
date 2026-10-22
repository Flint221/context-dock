import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { cloud } from "./cloud";
import { registry } from "./services/registry";
import type { Snapshot } from "./types";
import { processes } from "./services/processes";
vi.mock("./services/registry", () => ({
  registry: {
    snapshot: vi.fn(),
    add: vi.fn(),
    rename: vi.fn(),
    remove: vi.fn(),
    refresh: vi.fn(),
    open: vi.fn(),
    theme: vi.fn(),
    pickFolder: vi.fn(),
    subscribe: vi.fn(),
    tracking: vi.fn(),
    paths: vi.fn(),
    savePaths: vi.fn(),
  },
}));
vi.mock("./cloud", async (original) => ({
  ...(await original<typeof import("./cloud")>()),
  cloud: {
    view: vi.fn(),
    subscribe: vi.fn(),
    action: vi.fn(),
    connection: vi.fn(),
  },
}));
let state: Snapshot;
it("preserves page controls during status updates and resets the scene and scroll only on navigation", async () => {
  let notify: () => void = () => {};
  vi.mocked(registry.subscribe).mockImplementation(async (cb) => {
    notify = cb;
    return () => {};
  });
  const inventory = vi.spyOn(processes, "inventory").mockResolvedValue({
    processes: [],
    warnings: [],
    scannedAt: "2026-10-20T12:00:00Z",
  });
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole("button", { name: "Current processes" }));
  const search = await screen.findByRole("searchbox", {
    name: "Search apps or ports",
  });
  await waitFor(() => expect(inventory).toHaveBeenCalledTimes(1));
  await user.type(search, "5173");
  const scene = search.closest(".page-scene");
  const scrollCount = vi.mocked(window.scrollTo).mock.calls.length;
  await act(async () => notify());
  expect(screen.getByRole("searchbox")).toHaveValue("5173");
  expect(search.closest(".page-scene")).toBe(scene);
  expect(window.scrollTo).toHaveBeenCalledTimes(scrollCount);
  expect(inventory).toHaveBeenCalledTimes(1);
  await user.click(screen.getByRole("button", { name: "Settings" }));
  expect(scene).not.toBeInTheDocument();
  expect(window.scrollTo).toHaveBeenLastCalledWith({
    top: 0,
    left: 0,
    behavior: "instant",
  });
  inventory.mockRestore();
});
it("places Current processes above Settings and opens it independently of project registrations", async () => {
  const inventory = vi.spyOn(processes, "inventory").mockResolvedValue({
    processes: [],
    warnings: [],
    scannedAt: "2026-10-20T12:00:00Z",
  });
  const user = userEvent.setup();
  render(<App />);
  const nav = screen.getByRole("navigation", { name: "Main navigation" });
  const buttons = within(nav).getAllByRole("button");
  expect(buttons[2]).toHaveTextContent("Current processes");
  expect(buttons[3]).toHaveTextContent("Settings");
  await user.click(buttons[2]);
  expect(
    await screen.findByRole("button", { name: "Refresh processes" }),
  ).toBeInTheDocument();
  await waitFor(() => expect(inventory).toHaveBeenCalled());
  expect(
    screen.queryByRole("button", { name: "Add project" }),
  ).not.toBeInTheDocument();
  await user.click(buttons[3]);
  expect(
    screen.queryByRole("button", { name: "Refresh processes" }),
  ).not.toBeInTheDocument();
  inventory.mockRestore();
});
it("does not let an older registry read dismiss a newer folder-loss state", async () => {
  const p = {
    id: "p1",
    name: "Relocated project",
    path: "C:\\Old",
    createdAt: "",
    status: "available",
    lastScan: null,
    scanError: null,
    documentCount: 0,
    latestModified: null,
    trackingEnabled: false,
    archiveEnabled: false,
    trackingRevision: 0,
    folderState: "ready" as const,
  };
  state = { projects: [p], documents: [], theme: "system" };
  let notify: () => void = () => {};
  vi.mocked(registry.subscribe).mockImplementation(async (cb) => {
    notify = cb;
    return () => {};
  });
  let finish: (value: Snapshot) => void = () => {};
  vi.mocked(registry.snapshot).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  render(<App />);
  await waitFor(() => expect(registry.subscribe).toHaveBeenCalled());
  state = {
    ...state,
    projects: [
      { ...p, status: "unavailable", folderState: "needs_reassignment" },
    ],
  };
  await act(async () => notify());
  expect(
    await screen.findByText("Folders needing attention", { selector: "h2" }),
  ).toBeInTheDocument();
  await act(async () =>
    finish({ projects: [p], documents: [], theme: "system" }),
  );
  expect(
    screen.getByText("Folders needing attention", { selector: "h2" }),
  ).toBeInTheDocument();
});
beforeEach(() => {
  vi.mocked(registry.paths).mockResolvedValue({
    revision: 0,
    exclusions: [],
    entries: [],
    warnings: [],
    cloudPending: 0,
  });
  vi.mocked(registry.savePaths).mockResolvedValue(undefined);
  vi.clearAllMocks();
  vi.mocked(cloud.subscribe).mockResolvedValue(() => {});
  vi.mocked(cloud.connection).mockResolvedValue({
    saved: false,
    status: "disconnected",
    tested_at: null,
    error: null,
    usage: [],
  });
  vi.mocked(cloud.view).mockResolvedValue({
    enabled: 0,
    paused: 1,
    started: 0,
    auto_sync: 0,
    store_id: null,
    store_state: "none",
    store_name: "Test",
    status: "local_only",
    retainedFiles: 0,
    jobs: [],
    families: [],
    activity: [],
    connectionStatus: "disconnected",
    mode: "current",
    soft_limit: 52428800,
    pending: 0,
    indexed: 0,
    failed: 0,
    rawBytes: 0,
    usage_bytes: 0,
    error: null,
    created_at: null,
    last_sync: null,
  });
  vi.mocked(registry.subscribe).mockResolvedValue(() => {});
  vi.mocked(registry.tracking).mockResolvedValue({
    enabled: false,
    archiveEnabled: false,
    initialized: false,
    watchStatus: "off",
    error: null,
    families: [],
    sources: [],
    reviews: [],
    activity: [],
    operations: [],
  });
  state = { projects: [], documents: [], theme: "system" };
  vi.mocked(registry.snapshot).mockImplementation(async () =>
    structuredClone(state),
  );
});
it("guards real sidebar navigation and reloads saved folder rules on returning to the project", async () => {
  const user = userEvent.setup();
  state.projects = [
    {
      id: "p-tree",
      name: "Tree project",
      path: "C:\\Fixtures\\Tree",
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 0,
      latestModified: null,
      trackingEnabled: false,
      archiveEnabled: false,
      trackingRevision: 0,
      folderState: "ready",
    },
  ];
  let paths = {
    revision: 0,
    exclusions: [] as import("./types").PathRule[],
    entries: [{ path: "Drafts", kind: "folder" as const, status: "available" }],
    warnings: [],
    cloudPending: 0,
  };
  vi.mocked(registry.paths).mockImplementation(async () =>
    structuredClone(paths),
  );
  vi.mocked(registry.savePaths).mockImplementation(async (_p, rev, rules) => {
    paths = { ...paths, revision: rev + 1, exclusions: rules };
  });
  render(<App />);
  await user.click(
    await screen.findByRole("button", { name: /^Tree project C:/ }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("checkbox", { name: "Include Drafts" }),
    ).toBeEnabled(),
  );
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  await user.click(screen.getByRole("button", { name: "Settings" }));
  expect(
    screen.getByRole("dialog", { name: "Unsaved folder selection" }),
  ).toBeInTheDocument();
  expect(registry.savePaths).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  expect(
    await screen.findByRole("heading", { name: "Settings" }),
  ).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: /^Projects/ }));
  await user.click(
    await screen.findByRole("button", { name: /^Tree project C:/ }),
  );
  expect(
    await screen.findByRole("checkbox", { name: "Include Drafts" }),
  ).not.toBeChecked();
});
it("registers a real picker selection and opens its discovered documents", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.pickFolder).mockResolvedValue("C:\\Temporary project");
  vi.mocked(registry.add).mockImplementation(async (name, path) => {
    state.projects.push({
      id: "p1",
      name,
      path,
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 1,
      latestModified: null,
      trackingEnabled: false,
      archiveEnabled: false,
      trackingRevision: 0,
      folderState: "ready",
    });
    state.documents.push({
      id: "d1",
      projectId: "p1",
      documentId: "d1",
      name: "Context.md",
      relativePath: "docs/Context.md",
      absolutePath: path + "\\docs\\Context.md",
      kind: "markdown",
      size: 512,
      modifiedAt: null,
      lastObserved: "",
      status: "available",
    });
    return "p1";
  });
  render(<App />);
  await user.click(await screen.findByText("Add your first project"));
  await user.type(screen.getByLabelText("Project name"), "Cattle Tool");
  await user.click(screen.getByText("Browse"));
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  );
  expect(await screen.findByText("Context.md")).toBeInTheDocument();
  expect(registry.add).toHaveBeenCalledWith(
    "Cattle Tool",
    "C:\\Temporary project",
  );
  await user.click(screen.getByText("Project folder and options"));
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  expect(
    screen.getByText(/snapshots, version history and saved chats/),
  ).toBeInTheDocument();
  await user.click(screen.getByText("Cancel"));
  expect(registry.remove).not.toHaveBeenCalled();
});
it("handles cancellation and failed registration without closing the form", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.pickFolder).mockResolvedValue(null);
  render(<App />);
  await user.click(await screen.findByText("Add your first project"));
  await user.type(screen.getByLabelText("Project name"), "Test");
  await user.click(screen.getByText("Browse"));
  expect(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  ).toBeDisabled();
  vi.mocked(registry.pickFolder).mockResolvedValue("C:\\Test");
  await user.click(screen.getByText("Browse"));
  vi.mocked(registry.add).mockRejectedValue(
    "This folder is already registered.",
  );
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "already registered",
  );
  expect(screen.getByRole("dialog")).toBeInTheDocument();
});
it("persists theme changes and displays the real app version", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Add your first project");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  await user.selectOptions(screen.getByLabelText("Theme"), "dark");
  await waitFor(() => expect(registry.theme).toHaveBeenCalledWith("dark"));
  expect(screen.getByText("Version 0.9.0")).toBeInTheDocument();
});
it("renames, refreshes and removes only after the final confirmation", async () => {
  const user = userEvent.setup();
  state.projects = [
    {
      id: "p1",
      name: "Original",
      path: "C:\\Test",
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 0,
      latestModified: null,
      trackingEnabled: false,
      archiveEnabled: false,
      trackingRevision: 0,
      folderState: "ready",
    },
  ];
  vi.mocked(registry.rename).mockImplementation(async (_id, name) => {
    state.projects[0].name = name;
  });
  vi.mocked(registry.remove).mockImplementation(async () => {
    state.projects = [];
  });
  render(<App />);
  await user.click(await screen.findByRole("button", { name: /Original/ }));
  expect(registry.refresh).toHaveBeenCalledWith("p1");
  await user.click(screen.getByText("Project folder and options"));
  await user.click(screen.getByRole("button", { name: "Rename" }));
  await user.clear(screen.getByLabelText("Project name"));
  await user.type(screen.getByLabelText("Project name"), "Renamed");
  await user.click(screen.getByText("Save name"));
  await screen.findByRole("heading", { name: "Renamed" });
  expect(registry.rename).toHaveBeenCalledWith("p1", "Renamed");
  await user.click(screen.getByRole("button", { name: "Refresh documents" }));
  await screen.findByText("Document metadata refreshed.");
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  expect(registry.remove).not.toHaveBeenCalled();
  const snapshotsBeforeRemoval = vi.mocked(registry.snapshot).mock.calls.length;
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Remove project",
    }),
  );
  await screen.findByText("Add your first project");
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(registry.snapshot).toHaveBeenCalledTimes(snapshotsBeforeRemoval + 1);
  expect(registry.remove).toHaveBeenCalledWith("p1", false);
});

it("returns to All projects after removal and ignores an older read across later navigation", async () => {
  const user = userEvent.setup();
  const base = {
    path: "C:\\Test",
    createdAt: "",
    status: "available",
    lastScan: null,
    scanError: null,
    documentCount: 1,
    latestModified: null,
    trackingEnabled: false,
    archiveEnabled: false,
    trackingRevision: 0,
    folderState: "ready" as const,
  };
  state.projects = [
    { ...base, id: "p1", name: "Removed project" },
    { ...base, id: "p2", name: "Remaining project", path: "C:\\Other" },
  ];
  state.documents = state.projects.map((p) => ({
    id: `source-${p.id}`,
    projectId: p.id,
    documentId: `document-${p.id}`,
    name: `${p.name}.md`,
    relativePath: `${p.name}.md`,
    absolutePath: `${p.path}\\${p.name}.md`,
    kind: "markdown",
    size: 20,
    modifiedAt: null,
    lastObserved: "",
    status: "available",
  }));
  let notify: () => void = () => {};
  vi.mocked(registry.subscribe).mockImplementation(async (listener) => {
    notify = listener;
    return () => {};
  });
  vi.mocked(registry.remove).mockImplementation(async (id) => {
    state.projects = state.projects.filter((p) => p.id !== id);
    state.documents = state.documents.filter((d) => d.projectId !== id);
  });
  render(<App />);
  await user.click(
    await screen.findByRole("button", { name: /Removed project C:/ }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Refresh documents" }),
    ).toBeEnabled(),
  );
  const stale = structuredClone(state);
  let finishOldRead: (snapshot: Snapshot) => void = () => {};
  vi.mocked(registry.snapshot).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finishOldRead = resolve;
      }),
  );
  await act(async () => notify());
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  const dialog = screen.getByRole("dialog", { name: "Remove project?" });
  const confirm = within(dialog).getByRole("button", {
    name: "Remove project",
  });
  await waitFor(() => expect(confirm).toBeEnabled());
  await user.click(confirm);
  expect(
    await screen.findByRole("button", { name: /Remaining project C:/ }),
  ).toBeInTheDocument();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: /Removed project C:/ }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "All projects" }),
  ).not.toBeInTheDocument();
  await act(async () => finishOldRead(stale));
  await user.click(screen.getByRole("button", { name: "Dashboard" }));
  expect(
    screen.queryByRole("button", { name: /Removed project/ }),
  ).not.toBeInTheDocument();
  expect(screen.queryByText("Removed project.md")).not.toBeInTheDocument();
  expect(
    screen.getByText("Registered projects").parentElement,
  ).toHaveTextContent("1");
  expect(
    screen.getByText("Discovered documents").parentElement,
  ).toHaveTextContent("1");
  await user.click(screen.getByRole("button", { name: "Projects1" }));
  expect(
    screen.queryByRole("button", { name: /Removed project/ }),
  ).not.toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: /Remaining project C:/ }),
  ).toBeInTheDocument();
  expect(registry.remove).toHaveBeenCalledExactlyOnceWith("p1", false);
});

it("keeps one live tracking panel through updates and removes it on navigation", async () => {
  const user = userEvent.setup();
  state.projects = [
    {
      id: "p1",
      name: "Tracked project",
      path: "C:\\Test",
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 0,
      latestModified: null,
      trackingEnabled: true,
      archiveEnabled: false,
      trackingRevision: 0,
      folderState: "ready",
    },
  ];
  let notify: () => void = () => {};
  vi.mocked(registry.subscribe).mockImplementation(async (listener) => {
    notify = listener;
    return () => {};
  });
  vi.mocked(registry.tracking).mockResolvedValue({
    enabled: true,
    archiveEnabled: false,
    initialized: true,
    watchStatus: "watching",
    error: null,
    families: [],
    sources: [],
    reviews: [],
    activity: [],
    operations: [],
  });
  render(<App />);
  await user.click(
    await screen.findByRole("button", { name: /Tracked project/ }),
  );
  await screen.findByRole("button", { name: "Disable tracking" });
  for (let revision = 1; revision <= 8; revision++) {
    state.projects[0].trackingRevision = revision;
    await act(async () => notify());
    expect(
      screen.getAllByRole("heading", { name: "Document tracking" }),
    ).toHaveLength(1);
    expect(
      screen.getByRole("button", { name: "Refresh documents" }),
    ).toBeEnabled();
  }
  await user.click(screen.getByRole("button", { name: "Scan for changes" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Disable tracking" }),
    ).toBeEnabled(),
  );
  expect(screen.queryByText("Working…")).not.toBeInTheDocument();
  for (const page of ["Dashboard", "Projects1", "Settings"]) {
    await user.click(screen.getByRole("button", { name: page }));
    expect(
      screen.queryByRole("heading", { name: "Document tracking" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Refresh documents" }),
    ).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Projects1" }));
    await user.click(screen.getByRole("button", { name: /Tracked project/ }));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Refresh documents" }),
      ).toBeEnabled(),
    );
    expect(
      screen.getAllByRole("heading", { name: "Document tracking" }),
    ).toHaveLength(1);
    expect(
      screen.getByRole("heading", { name: "Registered folder" }),
    ).toBeInTheDocument();
  }
});

it("organizes project sections, guards unsaved tab changes and retains the folder tree", async () => {
  state.projects = [
    {
      id: "tabs",
      name: "Tabbed project",
      path: "C:\\Project",
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 1,
      latestModified: null,
      trackingEnabled: false,
      archiveEnabled: false,
      trackingRevision: 0,
      folderState: "ready",
    },
  ];
  let paths = {
    revision: 0,
    exclusions: [] as import("./types").PathRule[],
    entries: [
      { path: "Drafts", kind: "folder" as const, status: "available" },
      {
        path: "Drafts/Notes_v6.md",
        kind: "file" as const,
        status: "available",
      },
    ],
    warnings: [],
    cloudPending: 0,
  };
  vi.mocked(registry.paths).mockImplementation(async () =>
    structuredClone(paths),
  );
  vi.mocked(registry.savePaths).mockImplementation(
    async (_p, revision, exclusions) => {
      paths = { ...paths, revision: revision + 1, exclusions };
    },
  );
  const user = userEvent.setup();
  render(<App />);
  await user.click(
    await screen.findByRole("button", { name: /^Tabbed project C:/ }),
  );
  expect(
    screen.getByRole("heading", { name: "Tabbed project" }).closest("header"),
  ).toHaveClass("project-header");
  const files = screen.getByRole("tab", { name: "Files & tracking" });
  const cloudTab = screen.getByRole("tab", { name: "Cloud sync" });
  const ai = screen.getByRole("tab", { name: "Ask AI" });
  expect(files).toHaveAttribute("aria-selected", "true");
  const tree = await screen.findByRole("checkbox", { name: "Include Drafts" });
  await waitFor(() => expect(tree).toBeEnabled());
  expect(
    screen.getByRole("region", { name: "Project overview" }),
  ).toHaveTextContent("Selected files1");
  expect(
    screen.queryByRole("region", { name: "Cloud sync" }),
  ).not.toBeInTheDocument();
  await user.click(tree);
  await user.click(cloudTab);
  expect(
    screen.getByRole("dialog", { name: "Unsaved folder selection" }),
  ).toBeInTheDocument();
  expect(files).toHaveAttribute("aria-selected", "true");
  await user.click(screen.getByRole("button", { name: "Keep editing" }));
  expect(tree).not.toBeChecked();
  await user.click(cloudTab);
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  await waitFor(() =>
    expect(cloudTab).toHaveAttribute("aria-selected", "true"),
  );
  expect(
    screen.getByRole("region", { name: "Cloud sync" }),
  ).toBeInTheDocument();
  expect(
    screen.queryByRole("region", { name: "Ask project" }),
  ).not.toBeInTheDocument();
  await user.click(ai);
  expect(
    screen.getByRole("region", { name: "Ask project" }),
  ).toBeInTheDocument();
  await user.click(files);
  expect(screen.getByRole("checkbox", { name: "Include Drafts" })).toBe(tree);
  expect(tree).not.toBeChecked();
  expect(
    screen.getByRole("region", { name: "Project overview" }),
  ).toHaveTextContent("Selected files0");
  files.focus();
  await user.keyboard("{ArrowRight}");
  expect(cloudTab).toHaveFocus();
  expect(files).toHaveAttribute("aria-selected", "true");
  await user.keyboard("{Enter}");
  expect(cloudTab).toHaveAttribute("aria-selected", "true");
});
