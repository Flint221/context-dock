import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { cloud, type CloudView, type Plan, type Draft } from "../cloud";
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
const changePlan: Plan = {
  token: "change-review",
  files: 2,
  bytes: 200,
  additions: 1,
  removals: 0,
  storeName: "ContextDock - Test",
};
async function confirmedAction(
  _project: string | null,
  action: string,
  payload: unknown = {},
) {
  if (action === "preview") return changePlan as never;
  if (action === "apply") {
    const draft = (payload as { draft: Draft }).draft;
    view.enabled = Number(draft.enabled);
    view.auto_sync = Number(draft.automatic);
    view.mode = draft.mode;
    view.soft_limit = draft.softLimit;
    view.families.forEach((f) => {
      f.selected = Number(draft.documents.includes(f.id));
    });
  }
  if (["start", "sync", "resume"].includes(action)) {
    view.paused = 0;
    if (action === "start") view.started = 1;
  }
  return undefined as never;
}
beforeEach(() => {
  vi.clearAllMocks();
  view = {
    connectionStatus: "connected",
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
it("opens policy review immediately and restores the saved policy on cancel or close", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.paused = 0;
  view.families[0].selected = 1;
  vi.mocked(cloud.action).mockImplementation(async (_project, action) => {
    if (action === "preview") return changePlan as never;
    if (action === "apply") view.mode = "history";
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  for (const button of ["Cancel", "Close dialog"]) {
    await user.selectOptions(
      screen.getByLabelText("Version policy"),
      "history",
    );
    const modal = await screen.findByRole("dialog");
    expect(
      within(modal).getByRole("button", { name: "Confirm and sync" }),
    ).toBeDisabled();
    await user.click(within(modal).getByRole("button", { name: button }));
    expect(screen.getByLabelText("Version policy")).toHaveValue("current");
  }
  expect(vi.mocked(cloud.action).mock.calls.map((call) => call[1])).toEqual([
    "preview",
    "preview",
  ]);
  await user.selectOptions(screen.getByLabelText("Version policy"), "history");
  const modal = await screen.findByRole("dialog");
  expect(within(modal).getByRole("checkbox")).not.toBeChecked();
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm and sync" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(vi.mocked(cloud.action).mock.calls.map((call) => call[1])).toEqual([
    "preview",
    "preview",
    "preview",
    "apply",
    "sync",
  ]);
  expect(cloud.action).toHaveBeenCalledWith(
    "p1",
    "apply",
    expect.objectContaining({
      draft: expect.objectContaining({ mode: "history" }),
    }),
  );
});
it("reviews the upload limit once editing finishes, keeps errors visible, and cancels back to the saved limit", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  vi.mocked(cloud.action)
    .mockResolvedValueOnce(changePlan as never)
    .mockRejectedValueOnce("Review a fresh preview");
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  const input = screen.getByLabelText("Soft upload limit (MiB)");
  await user.clear(input);
  await user.type(input, "25");
  expect(cloud.action).not.toHaveBeenCalled();
  await user.keyboard("{Enter}");
  const modal = await screen.findByRole("dialog");
  expect(cloud.action).toHaveBeenCalledTimes(1);
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm cloud configuration" }),
  );
  expect(await within(modal).findByRole("alert")).toHaveTextContent(
    "configuration could not be confirmed",
  );
  expect(cloud.action).toHaveBeenCalledTimes(2);
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(input).toHaveValue(50);
});
it("opens review immediately when a saved document selection changes", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.families[0].selected = 1;
  vi.mocked(cloud.action).mockResolvedValue(changePlan as never);
  render(<CloudPanel project="p1" />);
  await user.click(await screen.findByLabelText(/Master Context/));
  const modal = await screen.findByRole("dialog");
  expect(cloud.action).toHaveBeenCalledWith("p1", "preview", {
    draft: expect.objectContaining({ documents: [] }),
  });
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(screen.getByLabelText(/Master Context/)).toBeChecked();
  expect(cloud.action).toHaveBeenCalledTimes(1);
});
it("asks to delete cloud data immediately and persists Off only after verified cleanup", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.connectionStatus = "error";
  view.store_state = "linked";
  vi.mocked(cloud.action).mockImplementation(async () => {
    view.enabled = 0;
    view.paused = 1;
    view.store_id = null;
    view.store_state = "none";
    view.status = "cloud_deleted";
    view.jobs = [];
    view.retainedFiles = 0;
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  const enable = await screen.findByLabelText(
    "Enable cloud indexing for this project",
  );
  await user.click(enable);
  let modal = screen.getByRole("dialog", { name: "Stop syncing?" });
  expect(within(modal).queryByRole("checkbox")).not.toBeInTheDocument();
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(enable).toBeChecked();
  await user.click(enable);
  modal = screen.getByRole("dialog");
  await user.click(within(modal).getByRole("button", { name: "Stop syncing" }));
  await waitFor(() => expect(enable).not.toBeChecked());
  expect(cloud.action).toHaveBeenCalledExactlyOnceWith(
    "p1",
    "delete_cloud",
    expect.objectContaining({ confirm: true }),
  );
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled();
});
it("keeps the stop dialog open on failure and leaves the saved checkbox enabled", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  vi.mocked(cloud.action).mockRejectedValueOnce("Database unavailable");
  render(<CloudPanel project="p1" />);
  const enable = await screen.findByLabelText(
    "Enable cloud indexing for this project",
  );
  await user.click(enable);
  const modal = screen.getByRole("dialog");
  await user.click(within(modal).getByRole("button", { name: "Stop syncing" }));
  expect(await within(modal).findByRole("alert")).toHaveTextContent(
    "Could not confirm syncing stopped",
  );
  expect(enable).toBeChecked();
  await user.click(within(modal).getByRole("button", { name: "Close dialog" }));
  expect(enable).toBeChecked();
});
it("confirms stopping automatic sync separately and reflects its saved paused state", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.auto_sync = 1;
  view.paused = 0;
  vi.mocked(cloud.action).mockImplementation(async () => {
    view.auto_sync = 0;
    view.paused = 1;
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  const automatic = await screen.findByLabelText(/Automatically sync newly/);
  await user.click(automatic);
  let modal = screen.getByRole("dialog", { name: "Stop automatic syncing?" });
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(automatic).toBeChecked();
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(automatic);
  modal = screen.getByRole("dialog");
  await user.click(
    within(modal).getByRole("button", { name: "Stop automatic syncing" }),
  );
  await waitFor(() => expect(automatic).not.toBeChecked());
  expect(cloud.action).toHaveBeenCalledExactlyOnceWith("p1", "stop_automatic");
  expect(
    screen.getByRole("button", { name: "Resume cloud sync" }),
  ).toBeEnabled();
  vi.mocked(cloud.action).mockReset().mockImplementation(confirmedAction);
  await user.click(automatic);
  modal = await screen.findByRole("dialog");
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm and sync" }),
  );
  await waitFor(() => expect(cloud.action).toHaveBeenCalledTimes(3));
  expect(vi.mocked(cloud.action).mock.calls.map((call) => call[1])).toEqual([
    "preview",
    "apply",
    "sync",
  ]);
});
it("requires confirmation and backend readback for pause and resume", async () => {
  view.enabled = 1;
  view.started = 1;
  view.paused = 0;
  const user = userEvent.setup();
  vi.mocked(cloud.action).mockImplementation(async (_p, action) => {
    if (action === "pause") view.paused = 1;
    if (action === "resume") view.paused = 0;
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  await user.click(
    await screen.findByRole("button", { name: "Pause cloud sync" }),
  );
  let modal = screen.getByRole("dialog", { name: "Pause cloud sync?" });
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Pause cloud sync" }));
  modal = screen.getByRole("dialog");
  await user.click(
    within(modal).getByRole("button", { name: "Pause cloud sync" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(screen.getByRole("status")).toHaveTextContent("paused — confirmed");
  await user.click(screen.getByRole("button", { name: "Resume cloud sync" }));
  modal = screen.getByRole("dialog", { name: "Resume cloud sync?" });
  await user.click(
    within(modal).getByRole("button", { name: "Resume cloud sync" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(screen.getByRole("status")).toHaveTextContent("resumed — confirmed");
});
it("reviews re-enabling an existing stopped project and resumes only after consent", async () => {
  const user = userEvent.setup();
  view.enabled = 0;
  view.started = 1;
  view.store_state = "linked";
  view.families[0].selected = 1;
  vi.mocked(cloud.action).mockImplementation(confirmedAction);
  render(<CloudPanel project="p1" />);
  await user.click(
    await screen.findByLabelText("Enable cloud indexing for this project"),
  );
  const modal = await screen.findByRole("dialog");
  expect(cloud.action).toHaveBeenCalledTimes(1);
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm and sync" }),
  );
  await waitFor(() => expect(cloud.action).toHaveBeenCalledTimes(3));
  expect(vi.mocked(cloud.action).mock.calls.map((call) => call[1])).toEqual([
    "preview",
    "apply",
    "sync",
  ]);
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
  vi.mocked(cloud.connection).mockResolvedValue({
    saved: true,
    status: "connected",
    tested_at: null,
    error: null,
    usage: [],
  });
  const local = vi.spyOn(Storage.prototype, "setItem");
  render(<OpenAIConnection />);
  const input = screen.getByLabelText("API key");
  await user.type(input, "test-secret-that-is-never-a-real-key");
  await user.click(screen.getByRole("button", { name: "Save and test key" }));
  expect(input).toHaveValue("");
  expect(cloud.save).toHaveBeenCalledWith(
    "test-secret-that-is-never-a-real-key",
  );
  expect(local).not.toHaveBeenCalled();
  expect(await screen.findByRole("status")).toHaveTextContent(
    "Key saved and connection verified",
  );
  local.mockRestore();
});
it("confirms global disconnect, blocks duplicate clicks, and verifies key removal", async () => {
  const user = userEvent.setup();
  let connection = {
    saved: true,
    status: "connected",
    tested_at: null,
    error: null,
    usage: [],
  };
  vi.mocked(cloud.connection).mockImplementation(async () =>
    structuredClone(connection),
  );
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => {
    finish = resolve;
  });
  vi.mocked(cloud.action).mockImplementation(async () => {
    await pending;
    connection = { ...connection, saved: false, status: "disconnected" };
    return undefined as never;
  });
  render(<OpenAIConnection />);
  const disconnect = await screen.findByRole("button", {
    name: "Disconnect OpenAI",
  });
  await waitFor(() => expect(disconnect).toBeEnabled());
  await user.click(disconnect);
  let modal = screen.getByRole("dialog");
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(disconnect);
  modal = screen.getByRole("dialog");
  const confirm = within(modal).getByRole("button", {
    name: "Disconnect OpenAI",
  });
  act(() => {
    fireEvent.click(confirm);
    fireEvent.click(confirm);
  });
  expect(cloud.action).toHaveBeenCalledTimes(1);
  expect(confirm).toBeDisabled();
  expect(within(modal).getByRole("status")).toHaveTextContent(
    "verifying key removal",
  );
  await act(async () => finish());
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(screen.getByRole("status")).toHaveTextContent(
    "OpenAI disconnected — confirmed",
  );
});
it("keeps disconnect errors visible when the key is still saved", async () => {
  vi.mocked(cloud.connection).mockResolvedValue({
    saved: true,
    status: "connected",
    tested_at: null,
    error: null,
    usage: [],
  });
  const user = userEvent.setup();
  render(<OpenAIConnection />);
  const disconnect = await screen.findByRole("button", {
    name: "Disconnect OpenAI",
  });
  await waitFor(() => expect(disconnect).toBeEnabled());
  await user.click(disconnect);
  const modal = screen.getByRole("dialog");
  await user.click(
    within(modal).getByRole("button", { name: "Disconnect OpenAI" }),
  );
  expect(await within(modal).findByRole("alert")).toHaveTextContent(
    "backend did not confirm",
  );
  expect(modal).toBeInTheDocument();
});
it("shows remote deletion confirmation without issuing deletion when cancelled", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.store_id = "vs_owned";
  view.store_state = "linked";
  render(<CloudPanel project="p1" />);
  await user.click(
    await screen.findByRole("button", { name: "Delete project cloud data" }),
  );
  const modal = screen.getByRole("dialog");
  expect(
    within(modal).getByRole("button", { name: "Confirm remote deletion" }),
  ).toBeDisabled();
  await user.click(within(modal).getByRole("button", { name: "Cancel" }));
  expect(cloud.action).not.toHaveBeenCalled();
});
it("confirms index deletion with its displayed name while sending the target ID separately", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.store_id = "vs_owned";
  view.store_state = "linked";
  view.store_name = "ContextDock - Project Ω";
  render(<CloudPanel project="p1" />);
  await user.click(
    await screen.findByRole("button", { name: "Delete project cloud data" }),
  );
  const modal = screen.getByRole("dialog");
  const input = within(modal).getByLabelText(
    "Type ContextDock - Project Ω to confirm",
  );
  const confirm = within(modal).getByRole("button", {
    name: "Confirm remote deletion",
  });
  await user.type(input, "DELETE vs_owned");
  expect(confirm).toBeDisabled();
  await user.clear(input);
  await user.type(input, view.store_name);
  expect(confirm).toBeEnabled();
  expect(cloud.action).not.toHaveBeenCalled();
  await user.click(confirm);
  await waitFor(() =>
    expect(cloud.action).toHaveBeenCalledWith(
      "p1",
      "delete_index",
      expect.objectContaining({
        confirmation: "ContextDock - Project Ω",
        storeId: "vs_owned",
      }),
    ),
  );
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
    screen.getByRole("button", { name: "Select all eligible documents" }),
  );
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
it("blocks an untested key before showing consent and offers an explicit project-side retry", async () => {
  const user = userEvent.setup();
  view.connectionStatus = "untested";
  vi.mocked(cloud.action).mockImplementation(async () => {
    view.connectionStatus = "connected";
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  await user.click(await screen.findByLabelText(/Master Context/));
  const review = screen.getByRole("button", { name: "Review cloud changes" });
  expect(review).toBeDisabled();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Test saved OpenAI key" }),
  );
  await waitFor(() => expect(review).toBeEnabled());
  expect(cloud.action).toHaveBeenCalledWith(null, "test");
  expect(cloud.action).not.toHaveBeenCalledWith(
    "p1",
    "apply",
    expect.anything(),
  );
});
it("shows why initial sync is disabled until a nonempty selection is saved, then closes consent on success", async () => {
  const user = userEvent.setup();
  vi.mocked(cloud.action).mockImplementation(async (_project, action) => {
    if (action === "preview")
      return {
        token: "ready",
        files: 1,
        bytes: 100,
        additions: 1,
        removals: 0,
        storeName: "Test",
      } as never;
    if (action === "apply") {
      view.enabled = 1;
      view.families[0].selected = 1;
      view.store_state = "pending";
    }
    return undefined as never;
  });
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  expect(
    screen.getByRole("button", { name: "Review cloud changes" }),
  ).toBeDisabled();
  expect(
    screen.getByText(/Select at least one eligible document below/),
  ).toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Select all eligible documents" }),
  );
  await user.click(
    screen.getByRole("button", { name: "Review cloud changes" }),
  );
  const modal = await screen.findByRole("dialog");
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm cloud configuration" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(
    screen.getByRole("button", { name: "Start Initial Sync" }),
  ).toBeEnabled();
  expect(
    screen.getByText(/Cloud configuration saved. Click Start Initial Sync/),
  ).toBeInTheDocument();
});
it("keeps key-test recovery available when save succeeds but the automatic connection test fails", async () => {
  const user = userEvent.setup();
  vi.mocked(cloud.connection).mockResolvedValue({
    saved: true,
    status: "error",
    error: "Test failed",
    tested_at: null,
    usage: [],
  });
  vi.mocked(cloud.action).mockRejectedValueOnce("OpenAI rejected the key");
  render(<OpenAIConnection />);
  await user.type(
    screen.getByLabelText("API key"),
    "synthetic-key-not-a-real-secret",
  );
  await user.click(screen.getByRole("button", { name: "Save and test key" }));
  expect(
    await screen.findByText("OpenAI rejected the key"),
  ).toBeInTheDocument();
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Test connection" }),
    ).toBeEnabled(),
  );
  expect(screen.getByLabelText("API key")).toHaveValue("");
});
it("explains queued configuration changes for an already started index without asking to restart initial sync", async () => {
  const user = userEvent.setup();
  view.enabled = 1;
  view.started = 1;
  view.families[0].selected = 1;
  vi.mocked(cloud.action)
    .mockResolvedValueOnce({
      token: "change",
      files: 1,
      bytes: 100,
      additions: 0,
      removals: 0,
      storeName: "Test",
    } as never)
    .mockResolvedValueOnce(undefined);
  render(<CloudPanel project="p1" />);
  await screen.findByLabelText(/Master Context/);
  await user.click(
    screen.getByRole("button", { name: "Review cloud changes" }),
  );
  const modal = screen.getByRole("dialog");
  await user.click(within(modal).getByRole("checkbox"));
  await user.click(
    within(modal).getByRole("button", { name: "Confirm cloud configuration" }),
  );
  expect(
    await screen.findByText(/Changes are queued; resume cloud sync/),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Start Initial Sync" }),
  ).toBeDisabled();
});
