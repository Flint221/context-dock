import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { drive, type DriveProject } from "../drive";
import { DrivePanel } from "./DrivePanel";

vi.mock("../drive", () => ({
  drive: {
    project: vi.fn(),
    configure: vi.fn(),
    publish: vi.fn(),
    subscribe: vi.fn(),
    createFolders: vi.fn(),
    recycling: vi.fn(),
    chooseProjectFolder: vi.fn(),
    recoverCleanup: vi.fn(),
  },
}));
let state: DriveProject;
beforeEach(() => {
  vi.resetAllMocks();
  state = {
    enabled: false,
    automatic: false,
    recycle: false,
    connected: true,
    accountId: null,
    folderId: "project-folder",
    rootReady: true,
    error: null,
    documents: [],
  };
  vi.mocked(drive.project).mockImplementation(async () =>
    structuredClone(state),
  );
  vi.mocked(drive.subscribe).mockResolvedValue(() => {});
});
it("keeps recovery visible while disconnected and retries without publishing", async () => {
  state.connected = false;
  state.cleanupError = "A safety file needs recovery.";
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "safety file needs recovery",
  );
  await user.click(screen.getByText("Optional local cleanup"));
  vi.mocked(drive.recoverCleanup).mockImplementation(async () => {
    state.cleanupError = null;
  });
  await user.click(
    screen.getByRole("button", { name: "Retry interrupted cleanup" }),
  );
  expect(drive.recoverCleanup).toHaveBeenCalledWith("sample");
  expect(
    await screen.findByText(/Interrupted cleanup checked/),
  ).toBeInTheDocument();
  expect(drive.publish).not.toHaveBeenCalled();
});
it("requires an explicit recycling choice and verifies its saved preference", async () => {
  state.enabled = true;
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(await screen.findByText("Optional local cleanup"));
  const option = screen.getByRole("checkbox", {
    name: "Recycle unchanged older local versions",
  });
  expect(option).not.toBeChecked();
  expect(drive.recycling).not.toHaveBeenCalled();
  vi.mocked(drive.recycling).mockResolvedValue(undefined);
  await user.click(option);
  expect(drive.recycling).toHaveBeenCalledWith("sample", true);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "preference did not save",
  );
  expect(option).not.toBeChecked();
});
it("does not let a late status read undo a verified recycling preference", async () => {
  state.enabled = true;
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(await screen.findByText("Optional local cleanup"));
  const old = structuredClone(state);
  let finish!: (value: DriveProject) => void;
  vi.mocked(drive.project).mockImplementationOnce(
    () =>
      new Promise<DriveProject>((r) => {
        finish = r;
      }),
  );
  const pending = vi.mocked(drive.subscribe).mock.calls[0][1]();
  vi.mocked(drive.recycling).mockImplementation(async (_project, value) => {
    state.recycle = value;
  });
  const choice = screen.getByRole("checkbox", {
    name: "Recycle unchanged older local versions",
  });
  await user.click(choice);
  await waitFor(() => expect(choice).toBeChecked());
  await act(async () => {
    finish(old);
    await pending;
  });
  expect(choice).toBeChecked();
});
it("can disable recycling during publication and keeps cleanup errors separate", async () => {
  state.enabled = true;
  state.recycle = true;
  state.cleanupError = "Older file changed; kept in place.";
  let finish!: () => void;
  vi.mocked(drive.publish).mockImplementation(
    () =>
      new Promise<void>((r) => {
        finish = r;
      }),
  );
  vi.mocked(drive.recycling).mockImplementation(async (_project, value) => {
    state.recycle = value;
  });
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(await screen.findByText("Optional local cleanup"));
  expect(screen.getByRole("alert")).toHaveTextContent(
    "cleanup needs attention",
  );
  await user.click(screen.getByRole("button", { name: "Publish latest" }));
  const option = screen.getByRole("checkbox", {
    name: "Recycle unchanged older local versions",
  });
  expect(option).toBeEnabled();
  await user.click(option);
  await waitFor(() => expect(option).not.toBeChecked());
  expect(drive.recycling).toHaveBeenCalledWith("sample", false);
  await act(async () => finish());
});
it("enables automatic publication without a duplicate immediate manual request", async () => {
  vi.mocked(drive.configure).mockImplementation(
    async (_id, enabled, automatic) => {
      state.enabled = enabled;
      state.automatic = automatic;
    },
  );
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(
    await screen.findByRole("checkbox", {
      name: "Publish this project to Google Drive",
    }),
  );
  expect(drive.configure).toHaveBeenCalledWith("sample", true, true);
  expect(drive.publish).not.toHaveBeenCalled();
  expect(
    await screen.findByText(/Drive publication enabled/),
  ).toBeInTheDocument();
});
it("requires a project-folder choice and verifies picker adoption before publication", async () => {
  state.enabled = true;
  state.folderId = null;
  state.folderReady = false;
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  expect(
    await screen.findByRole("button", { name: "Publish latest" }),
  ).toBeDisabled();
  vi.mocked(drive.chooseProjectFolder).mockImplementation(async () => {
    state.folderId = "existing-folder";
    state.folderReady = true;
  });
  await user.click(
    screen.getByRole("button", {
      name: "Choose existing Drive project folder",
    }),
  );
  expect(drive.chooseProjectFolder).toHaveBeenCalledWith("sample");
  expect(
    await screen.findByText(/Existing project folder selected/),
  ).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Publish latest" })).toBeEnabled();
  expect(drive.createFolders).not.toHaveBeenCalled();
});
it("only reports publication success after verified documents are returned", async () => {
  state.enabled = true;
  vi.mocked(drive.publish).mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(
    await screen.findByRole("button", { name: "Publish latest" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(/did not confirm/);
  expect(
    screen.queryByText("Latest files published and verified."),
  ).not.toBeInTheDocument();
});
it("keeps the previous receipt visible after a failed replacement", async () => {
  state.enabled = true;
  state.documents = [
    {
      documentId: "doc",
      fileId: "same-file",
      verifiedHash: "old",
      verifiedName: "ContextDock_Context_v1.md",
      verifiedSize: 3,
      verifiedAt: "2026-11-09T00:00:00Z",
      pendingName: null,
      state: "synced",
      error: null,
    },
  ];
  vi.mocked(drive.publish).mockImplementation(async () => {
    state.documents[0].pendingName = "ContextDock_Context_v2.md";
    state.documents[0].state = "offline";
    throw new Error("offline; upload remains queued");
  });
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(
    await screen.findByRole("button", { name: "Publish latest" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "upload remains queued",
  );
  expect(
    await screen.findByText("ContextDock_Context_v2.md"),
  ).toBeInTheDocument();
  expect(state.documents[0].verifiedHash).toBe("old");
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Publish latest" }),
    ).toBeEnabled(),
  );
});
it("permits pausing a disconnected project while blocking new publication", async () => {
  state.enabled = true;
  state.connected = false;
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  expect(
    await screen.findByRole("button", { name: "Publish latest" }),
  ).toBeDisabled();
  vi.mocked(drive.configure).mockImplementation(async (_id, enabled) => {
    state.enabled = enabled;
  });
  await user.click(
    screen.getByRole("checkbox", {
      name: "Publish this project to Google Drive",
    }),
  );
  expect(
    await screen.findByText(/Drive publication paused/),
  ).toBeInTheDocument();
});

it("requires an explicit root setup before publication and never uploads on folder creation", async () => {
  state.enabled = true;
  state.rootReady = false;
  vi.mocked(drive.createFolders).mockImplementation(async () => {
    state.rootReady = true;
    state.folderId = "sample-folder";
  });
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  expect(
    await screen.findByRole("button", { name: "Publish latest" }),
  ).toBeDisabled();
  await user.click(
    screen.getByRole("button", { name: "Create Drive folders" }),
  );
  expect(
    await screen.findByText(/Drive folders are ready/),
  ).toBeInTheDocument();
  expect(drive.publish).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "Publish latest" })).toBeEnabled();
});

it("can pause while a manual publication is in flight", async () => {
  state.enabled = true;
  let stop: (reason: Error) => void = () => {};
  vi.mocked(drive.publish).mockReturnValue(
    new Promise((_resolve, reject) => {
      stop = reject;
    }),
  );
  vi.mocked(drive.configure).mockImplementation(async (_project, enabled) => {
    state.enabled = enabled;
  });
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  await user.click(
    await screen.findByRole("button", { name: "Publish latest" }),
  );
  expect(
    screen.getByRole("button", { name: "Pause publication" }),
  ).toBeEnabled();
  await user.click(screen.getByRole("button", { name: "Pause publication" }));
  await waitFor(() =>
    expect(
      screen.getByRole("checkbox", {
        name: "Publish this project to Google Drive",
      }),
    ).not.toBeChecked(),
  );
  await act(async () =>
    stop(new Error("Publication paused; the pending item was retained.")),
  );
  expect(
    screen.queryByText("Latest files published and verified."),
  ).not.toBeInTheDocument();
});

it("makes a migrated manual setting visible and enables automation only when requested", async () => {
  state.enabled = true;
  state.automatic = false;
  vi.mocked(drive.configure).mockImplementation(
    async (_project, enabled, automatic) => {
      state.enabled = enabled;
      state.automatic = automatic;
    },
  );
  const user = userEvent.setup();
  render(<DrivePanel project="sample" />);
  expect(
    await screen.findByRole("button", { name: "Enable automatic publication" }),
  ).toBeInTheDocument();
  expect(drive.configure).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Enable automatic publication" }),
  );
  expect(
    await screen.findByText(/Automatic publication enabled/),
  ).toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "Enable automatic publication" }),
  ).not.toBeInTheDocument();
});
