import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { cloud } from "./cloud";
import { registry } from "./services/registry";
import type { Snapshot } from "./types";
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
  expect(screen.getByText("Version 0.6.0 · Milestone 6")).toBeInTheDocument();
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
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Remove project",
    }),
  );
  await screen.findByText("Add your first project");
  expect(registry.remove).toHaveBeenCalledWith("p1", false);
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
