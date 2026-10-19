import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { cloud, type CloudView, type Plan } from "../cloud";
import { CloudPanel } from "./CloudPanel";
import { OpenAIConnection } from "./OpenAIConnection";
vi.mock("../cloud", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../cloud")>()),
  cloud: {
    view: vi.fn(),
    connection: vi.fn(),
    action: vi.fn(),
    save: vi.fn(),
    subscribe: vi.fn(),
  },
}));
let view: CloudView;
beforeEach(() => {
  vi.clearAllMocks();
  view = {
    enabled: 0,
    paused: 1,
    auto_sync: 0,
    started: 0,
    mode: "current",
    soft_limit: 52428800,
    store_id: null,
    store_name: "ContextDock - Test",
    store_state: "none",
    status: "local_only",
    error: null,
    created_at: null,
    last_sync: null,
    usage_bytes: null,
    families: [
      {
        id: "d1",
        name: "Master Context",
        eligible: 1,
        selected: 0,
        currentNumber: 1,
        cloudNumber: null,
        versionCount: 1,
        rawBytes: 100,
      },
    ],
    jobs: [],
    activity: [],
    indexed: 0,
    pending: 0,
    failed: 0,
    rawBytes: 0,
    retainedFiles: 0,
  };
  vi.mocked(cloud.view).mockImplementation(async () => structuredClone(view));
  vi.mocked(cloud.subscribe).mockResolvedValue(() => {});
  vi.mocked(cloud.action).mockResolvedValue(undefined);
  vi.mocked(cloud.save).mockResolvedValue(undefined);
  vi.mocked(cloud.connection).mockResolvedValue({
    saved: false,
    status: "disconnected",
    tested_at: null,
    error: null,
    usage: [],
  });
});
it("requires a reviewed explicit selection and checked consent before apply; never starts initial uploads automatically", async () => {
  const user = userEvent.setup();
  const plan: Plan = {
    token: "review-token",
    files: 1,
    bytes: 100,
    additions: 1,
    removals: 0,
    storeName: view.store_name,
  };
  vi.mocked(cloud.action)
    .mockResolvedValueOnce(plan as never)
    .mockResolvedValueOnce(undefined);
  render(<CloudPanel project="p1" />);
  await user.click(await screen.findByLabelText(/Master Context/));
  await user.click(
    screen.getByRole("button", { name: "Review cloud changes" }),
  );
  const modal = await screen.findByRole("dialog");
  const confirm = within(modal).getByRole("button", {
    name: "Confirm cloud configuration",
  });
  expect(confirm).toBeDisabled();
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(confirm);
  await waitFor(() => expect(cloud.action).toHaveBeenCalledTimes(2));
  expect(cloud.action).toHaveBeenLastCalledWith(
    "p1",
    "apply",
    expect.objectContaining({
      token: "review-token",
      consent: true,
      draft: expect.objectContaining({ documents: ["d1"], automatic: false }),
    }),
  );
  expect(cloud.action).not.toHaveBeenCalledWith(
    "p1",
    "start",
    expect.anything(),
  );
});
it("keeps local use available when cloud status fails and unlocks cloud controls after a rejected action", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.paused = 0;
  view.store_state = "linked";
  vi.mocked(cloud.action).mockRejectedValueOnce("Network unavailable");
  render(<CloudPanel project="p1" />);
  const button = await screen.findByRole("button", { name: "Sync Now" });
  await waitFor(() => expect(button).toBeEnabled());
  await user.click(button);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Network unavailable",
  );
  expect(button).toBeEnabled();
});
it("rejects stale responses after project panel unmount", async () => {
  let finish: (value: CloudView) => void = () => {};
  vi.mocked(cloud.view).mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve;
    }),
  );
  const old = render(<CloudPanel project="old" />);
  old.unmount();
  render(<CloudPanel project="new" />);
  await screen.findByLabelText(/Master Context/);
  await act(async () => finish({ ...view, store_id: "vs_old" }));
  expect(screen.queryByText("vs_old")).not.toBeInTheDocument();
  expect(
    screen.getAllByRole("heading", { name: "Cloud indexing" }),
  ).toHaveLength(1);
});
it("clears the uncontrolled password field immediately and does not put a key into browser storage", async () => {
  const user = userEvent.setup();
  const local = vi.spyOn(Storage.prototype, "setItem");
  render(<OpenAIConnection />);
  const input = screen.getByLabelText("API key");
  await user.type(input, "test-secret-that-is-never-a-real-key");
  await user.click(screen.getByRole("button", { name: "Save / replace key" }));
  expect(input).toHaveValue("");
  expect(cloud.save).toHaveBeenCalledWith(
    "test-secret-that-is-never-a-real-key",
  );
  expect(local).not.toHaveBeenCalled();
  expect(await screen.findByRole("status")).toHaveTextContent("Key saved");
  local.mockRestore();
});
it("shows remote deletion confirmation without issuing deletion when cancelled", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.store_id = "vs_owned";
  view.store_state = "linked";
  render(<CloudPanel project="p1" />);
  await user.click(
    await screen.findByRole("button", { name: "Delete project cloud index" }),
  );
  const modal = screen.getByRole("dialog");
  expect(
    within(modal).getByRole("button", { name: "Confirm remote deletion" }),
  ).toBeDisabled();
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(cloud.action).not.toHaveBeenCalled();
});
it("shows an actionable configuration failure inside its consent dialog", async () => {
  const user = userEvent.setup();
  vi.mocked(cloud.action)
    .mockResolvedValueOnce({
      token: "stale",
      files: 0,
      bytes: 0,
      additions: 0,
      removals: 0,
      storeName: "Test",
    } as never)
    .mockRejectedValueOnce("Review a fresh preview");
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  await user.click(
    screen.getByRole("button", { name: "Review cloud changes" }),
  );
  const dialog = screen.getByRole("dialog");
  await user.click(within(dialog).getByRole("checkbox"));
  await user.click(
    within(dialog).getByRole("button", { name: "Confirm cloud configuration" }),
  );
  expect(await within(dialog).findByRole("alert")).toHaveTextContent(
    "Review a fresh preview",
  );
  expect(within(dialog).getByRole("button", { name: "Cancel" })).toBeEnabled();
});
it("allows an explicitly confirmed association cleanup while sync is paused without resuming uploads", async () => {
  const user = userEvent.setup();
  view.enabled = 0;
  view.paused = 1;
  view.store_id = "vs_owned";
  view.store_state = "linked";
  view.jobs = [
    {
      id: "j1",
      version_id: "v1",
      file_id: "file-owned",
      familyName: "Master Context",
      number: 1,
      state: "indexed",
      error: null,
      usage_bytes: 42,
      rawBytes: 100,
    },
  ];
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  await user.click(
    screen.getByRole("button", { name: "View Cloud Index / Test Search" }),
  );
  await user.click(screen.getByRole("button", { name: "Remove association" }));
  const modal = screen.getByRole("dialog", { name: "Confirm remote deletion" });
  await user.click(
    within(modal).getByRole("button", { name: "Confirm remote deletion" }),
  );
  await waitFor(() =>
    expect(cloud.action).toHaveBeenCalledWith(
      "p1",
      "remove_now",
      expect.objectContaining({ version: "v1", confirm: true }),
    ),
  );
  expect(cloud.action).not.toHaveBeenCalledWith(
    "p1",
    "resume",
    expect.anything(),
  );
});
