import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { DesktopSettings } from "./DesktopSettings";
import { PendingApprovals } from "./PendingApprovals";
import {
  desktop,
  type DesktopStatus,
  type PendingFile,
} from "../services/desktop";
vi.mock("../services/desktop", () => ({
  desktop: {
    native: vi.fn(() => true),
    read: vi.fn(),
    setting: vi.fn(),
    testNotification: vi.fn(),
    subscribe: vi.fn(),
    pending: vi.fn(),
    preview: vi.fn(),
    apply: vi.fn(),
  },
}));
let status: DesktopStatus;
let pending: PendingFile[];
beforeEach(() => {
  vi.clearAllMocks();
  status = {
    keepRunning: true,
    startupRegistered: false,
    startupEnabled: false,
    approvalNotifications: true,
    problemNotifications: true,
    uploadsPaused: false,
    pending: 2,
    error: null,
  };
  pending = [
    {
      projectId: "p1",
      projectName: "Test project",
      path: "Drafts/New.md",
      kind: "markdown",
      size: 420,
      status: "available",
      modified: null,
    },
    {
      projectId: "p1",
      projectName: "Test project",
      path: "Other.docx",
      kind: "word",
      size: 1024,
      status: "available",
      modified: null,
    },
  ];
  vi.mocked(desktop.subscribe).mockResolvedValue(() => {});
  vi.mocked(desktop.read).mockImplementation(async () => ({ ...status }));
  vi.mocked(desktop.testNotification).mockResolvedValue(
    "Test notification submitted to Windows. Check Notification Center.",
  );
  vi.mocked(desktop.pending).mockImplementation(async () =>
    structuredClone(pending),
  );
  vi.mocked(desktop.setting).mockImplementation(async (key, value) => {
    if (key === "startup") status.startupRegistered = value;
    else Object.assign(status, { [key]: value });
  });
  vi.mocked(desktop.preview).mockImplementation(async (request) => ({
    ...request,
    token: "review-token",
    projectName: "Test project",
    revision: 3,
    trackingEnabled: 1,
    cloudEnabled: 0,
    automatic: 0,
  }));
  vi.mocked(desktop.apply).mockImplementation(async (request) => {
    pending = pending.filter((f) => !request.paths.includes(f.path));
  });
});
it("tests delivery without changing preferences or approvals and surfaces Windows failures", async () => {
  const user = userEvent.setup();
  render(<DesktopSettings />);
  const button = await screen.findByRole("button", {
    name: "Send test notification",
  });
  await user.click(button);
  expect(await screen.findByRole("status")).toHaveTextContent(
    "submitted to Windows",
  );
  expect(desktop.setting).not.toHaveBeenCalled();
  expect(desktop.apply).not.toHaveBeenCalled();
  vi.mocked(desktop.testNotification).mockRejectedValueOnce(
    new Error(
      "Windows notifications are disabled. Enable ContextDock in Windows notification settings.",
    ),
  );
  await user.click(button);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Windows notifications are disabled",
  );
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
  expect(button).not.toBeDisabled();
});
it("reads persisted desktop controls and supports reversing each preference", async () => {
  const user = userEvent.setup();
  render(<DesktopSettings />);
  for (const label of [
    "Keep running when closed",
    "Start with Windows, hidden in the tray",
    "New files awaiting approval",
    "Tracking, folder and cloud problems",
  ]) {
    const box = await screen.findByRole("checkbox", { name: label });
    const initial = (box as HTMLInputElement).checked;
    await user.click(box);
    await waitFor(() => expect(box).not.toBeDisabled());
    expect((box as HTMLInputElement).checked).toBe(!initial);
    await user.click(box);
    await waitFor(() =>
      expect((box as HTMLInputElement).checked).toBe(initial),
    );
  }
  await user.click(
    screen.getByRole("button", { name: "Pause background uploads" }),
  );
  await user.click(
    await screen.findByRole("button", { name: "Resume background uploads" }),
  );
  await screen.findByRole("button", { name: "Pause background uploads" });
  expect(desktop.setting).toHaveBeenCalledWith("uploadsPaused", true);
  expect(desktop.setting).toHaveBeenCalledWith("uploadsPaused", false);
});
it("reports Windows startup overrides without silently enabling them", async () => {
  status.startupRegistered = true;
  status.startupEnabled = false;
  render(<DesktopSettings />);
  expect(await screen.findByText(/Windows has disabled startup/)).toHaveClass(
    "error-text",
  );
  expect(desktop.setting).not.toHaveBeenCalled();
});
it("requires explicit selection and confirmation before approving only selected files", async () => {
  const user = userEvent.setup();
  render(<PendingApprovals project="p1" openProject={vi.fn()} />);
  const checkbox = await screen.findByRole("checkbox", {
    name: "Select Drafts/New.md",
  });
  expect(checkbox).not.toBeChecked();
  expect(
    screen.getByRole("button", { name: "Approve selected (0)" }),
  ).toBeDisabled();
  await user.click(checkbox);
  expect(desktop.apply).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Approve selected (1)" }),
  );
  let dialog = await screen.findByRole("dialog", {
    name: "Approve new files?",
  });
  expect(dialog).toHaveTextContent("cloud sync is off");
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(desktop.apply).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Approve selected (1)" }),
  );
  dialog = await screen.findByRole("dialog");
  await user.click(
    within(dialog).getByRole("button", { name: "Confirm and save" }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole("checkbox", { name: "Select Drafts/New.md" }),
    ).not.toBeInTheDocument(),
  );
  expect(desktop.apply).toHaveBeenCalledWith(
    { project: "p1", paths: ["Drafts/New.md"], action: "approve" },
    "review-token",
  );
  expect(
    screen.getByRole("checkbox", { name: "Select Other.docx" }),
  ).not.toBeChecked();
});
it("ignores a confirmed batch and shows stale-review errors without pretending success", async () => {
  const user = userEvent.setup();
  render(<PendingApprovals project={null} openProject={vi.fn()} />);
  await user.click(
    await screen.findByRole("button", { name: "Select available files" }),
  );
  await user.click(screen.getByRole("button", { name: "Ignore selected" }));
  const dialog = await screen.findByRole("dialog", {
    name: "Ignore new files?",
  });
  expect(dialog).toHaveTextContent(
    "Source files and existing local history are kept",
  );
  vi.mocked(desktop.apply).mockRejectedValueOnce(
    new Error("Files changed. Reload and review again."),
  );
  await user.click(
    within(dialog).getByRole("button", { name: "Confirm and save" }),
  );
  expect(await screen.findByRole("alert")).toHaveClass("error-text");
  expect(screen.queryByText(/Review saved/)).not.toBeInTheDocument();
  expect(screen.getAllByRole("checkbox")).toHaveLength(2);
});
