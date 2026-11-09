import { render, screen, waitFor } from "@testing-library/react";
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
    folderId: null,
    rootReady: true,
    error: null,
    documents: [],
  };
  vi.mocked(drive.project).mockImplementation(async () =>
    structuredClone(state),
  );
  vi.mocked(drive.subscribe).mockResolvedValue(() => {});
});
it("enables manual publication without starting an upload", async () => {
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
  expect(drive.configure).toHaveBeenCalledWith("sample", true, false);
  expect(drive.publish).not.toHaveBeenCalled();
  expect(
    await screen.findByText(/Drive publication enabled/),
  ).toBeInTheDocument();
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
