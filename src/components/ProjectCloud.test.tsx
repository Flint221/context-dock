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
import { ProjectCloud } from "./ProjectCloud";
import { cloud, type CloudView } from "../cloud";
import { chat, type Chat } from "../chat";
vi.mock("../cloud", async (original) => ({
  ...(await original<typeof import("../cloud")>()),
  cloud: { view: vi.fn(), action: vi.fn(), subscribe: vi.fn() },
}));
vi.mock("../chat", () => ({ chat: { action: vi.fn() } }));
let view: CloudView;
let chats: Chat[];
let changed: () => void;
it("blocks new questions about missing selected files while saved chats remain readable", async () => {
  connected();
  chats = [structuredClone(saved)];
  view.localReady = false;
  view.localError =
    "Selected project files are missing. Restore them and refresh documents.";
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  expect(await screen.findByText("SQLite [1].")).toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Your question" })).toBeDisabled();
  expect(
    screen.getByText(/Selected project files are missing/),
  ).toBeInTheDocument();
  view.localReady = true;
  view.localError = null;
  await act(async () => changed());
  await waitFor(() =>
    expect(
      screen.getByRole("textbox", { name: "Your question" }),
    ).toBeEnabled(),
  );
  await user.type(
    screen.getByRole("textbox", { name: "Your question" }),
    "Ready again",
  );
  expect(chat.action).not.toHaveBeenCalledWith(
    "p1",
    "ask",
    expect.anything(),
    expect.anything(),
  );
});
const saved: Chat = {
  id: "c1",
  title: "Database question",
  project_id: "p1",
  created_at: "now",
  updated_at: "now",
  turns: [
    {
      id: "t1",
      question: "Which database?",
      answer: "SQLite [1].",
      state: "completed",
      sources: [
        {
          citation: 1,
          fileId: "file1",
          familyName: "Master Context",
          sourceName: "Master_Context.md",
          versionId: "v1",
          number: 1,
          score: 0.9,
          excerpt: "SQLite database",
        },
      ],
      created_at: "now",
      error: null,
    },
  ],
} as Chat;
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
    store_name: "Test project",
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
  chats = [];
  vi.mocked(cloud.view).mockImplementation(async () => structuredClone(view));
  vi.mocked(cloud.subscribe).mockImplementation(async (cb) => {
    changed = cb;
    return () => {};
  });
  vi.mocked(cloud.action).mockImplementation(async (_p, action, payload) => {
    if (action === "preview")
      return {
        token: "review",
        files: 1,
        bytes: 100,
        additions: 1,
        removals: 0,
        storeName: "Test project",
      } as never;
    if (action === "apply") {
      const draft = (
        payload as {
          draft: {
            automatic: boolean;
            mode: "current" | "history";
            softLimit: number;
          };
        }
      ).draft;
      view.enabled = 1;
      view.auto_sync = Number(draft.automatic);
      view.mode = draft.mode;
      view.soft_limit = draft.softLimit;
      view.families[0].selected = 1;
    }
    if (action === "start") connected();
    if (action === "delete_cloud") {
      view.enabled = 0;
      view.paused = 1;
      view.store_id = null;
      view.store_state = "none";
      view.status = "cloud_deleted";
      view.jobs = [];
      view.retainedFiles = 0;
    }
    return undefined as never;
  });
  vi.mocked(chat.action).mockImplementation(async (_p, action, payload) => {
    const data = payload as { id?: string; question?: string };
    if (action === "list") return structuredClone(chats) as never;
    if (action === "get")
      return structuredClone(chats.find((c) => c.id === data.id)) as never;
    if (action === "create") {
      const c = { ...structuredClone(saved), turns: [], title: "New chat" };
      chats.push(c);
      return c as never;
    }
    if (action === "send") {
      chats[0] = structuredClone(saved);
      return chats[0] as never;
    }
    if (action === "delete") chats = chats.filter((c) => c.id !== data.id);
    return undefined as never;
  });
});
function connected() {
  view.enabled = 1;
  view.started = 1;
  view.paused = 0;
  view.store_id = "vs_test";
  view.store_state = "linked";
  view.families[0].selected = 1;
  view.families[0].cloudNumber = 1;
  view.indexed = 1;
}
it.each(["empty", "ineligible"])(
  "preserves an %s saved selection instead of silently selecting all documents on re-enable",
  async (kind) => {
    view.selectionSaved = true;
    view.status = "cloud_deleted";
    if (kind === "ineligible")
      view.families.push({
        ...view.families[0],
        id: "old-selection",
        name: "Needs review",
        selected: 1,
        eligible: 0,
      });
    const user = userEvent.setup();
    render(<ProjectCloud project="p1" />);
    await user.click(await screen.findByRole("switch", { name: "Cloud sync" }));
    expect(
      await screen.findByText(
        /saved document selection is empty or needs review/,
      ),
    ).toBeInTheDocument();
    expect(cloud.action).not.toHaveBeenCalled();
    expect(
      screen.getByRole("switch", { name: "Cloud sync" }),
    ).not.toBeChecked();
  },
);
it("re-enables only the saved subset rather than newly eligible unrelated families", async () => {
  view.selectionSaved = true;
  view.status = "cloud_deleted";
  view.families[0].selected = 1;
  view.families.push({
    ...view.families[0],
    id: "not-selected",
    name: "Unselected",
    selected: 0,
  });
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  await user.click(await screen.findByRole("switch", { name: "Cloud sync" }));
  await screen.findByRole("dialog");
  expect(cloud.action).toHaveBeenCalledExactlyOnceWith("p1", "preview", {
    draft: expect.objectContaining({ documents: ["d1"] }),
  });
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }),
  );
  expect(vi.mocked(cloud.action).mock.calls).toHaveLength(1);
});
it("keeps cloud off until one confirmation, then starts sync and enables a simple question", async () => {
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeEnabled());
  expect(screen.queryByText("Create baseline")).not.toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Your question" })).toBeDisabled();
  await user.click(toggle);
  const dialog = await screen.findByRole("dialog", {
    name: "Enable Cloud sync and AI?",
  });
  expect(toggle).not.toBeChecked();
  await user.click(
    within(dialog).getByRole("button", { name: "Enable Cloud sync and AI" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(toggle).toBeChecked();
  expect(cloud.action).toHaveBeenCalledWith("p1", "start");
  expect(cloud.action).toHaveBeenCalledWith(
    "p1",
    "apply",
    expect.objectContaining({
      consent: true,
      draft: expect.objectContaining({ automatic: true, documents: ["d1"] }),
    }),
  );
  await user.type(
    screen.getByRole("textbox", { name: "Your question" }),
    "Which database?",
  );
  await user.click(screen.getByRole("button", { name: "Send" }));
  expect(await screen.findByText("SQLite [1].")).toBeInTheDocument();
  expect(chat.action).toHaveBeenCalledWith(
    "p1",
    "send",
    {
      id: "c1",
      question: "Which database?",
    },
    expect.any(Function),
  );
});
it("cancel restores the actual switch and turning off blocks new questions", async () => {
  connected();
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeChecked());
  await user.click(toggle);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }),
  );
  expect(toggle).toBeChecked();
  expect(cloud.action).not.toHaveBeenCalledWith("p1", "delete_cloud");
  await user.click(toggle);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Turn off Cloud sync",
    }),
  );
  await waitFor(() => expect(toggle).not.toBeChecked());
  expect(screen.getByRole("textbox", { name: "Your question" })).toBeDisabled();
});
it("reads and deletes only a local saved chat while cloud sync is off", async () => {
  chats = [structuredClone(saved)];
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  expect(await screen.findByText("SQLite [1].")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Delete chat" }));
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Delete local chat",
    }),
  );
  await waitFor(() =>
    expect(screen.queryByText("SQLite [1].")).not.toBeInTheDocument(),
  );
  expect(chat.action).toHaveBeenCalledWith("p1", "delete", { id: "c1" });
  expect(cloud.action).not.toHaveBeenCalled();
});
it("waits for the latest indexed version and unlocks after a sync event", async () => {
  connected();
  view.families[0].currentNumber = 2;
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  await screen.findByText("On · Preparing your documents");
  expect(screen.getByRole("textbox", { name: "Your question" })).toBeDisabled();
  view.families[0].cloudNumber = 2;
  await act(async () => changed());
  await waitFor(() =>
    expect(
      screen.getByRole("textbox", { name: "Your question" }),
    ).toBeEnabled(),
  );
  await user.click(screen.getByRole("button", { name: "New chat" }));
  expect(chat.action).not.toHaveBeenCalledWith("p1", "send", expect.anything());
});
it("restores independent chats after reopening without generating another answer", async () => {
  connected();
  chats = [
    structuredClone(saved),
    {
      ...structuredClone(saved),
      id: "c2",
      title: "Another conversation",
      turns: [],
    },
  ];
  const user = userEvent.setup();
  const first = render(<ProjectCloud project="p1" />);
  await screen.findByText("SQLite [1].");
  await user.click(
    screen.getByRole("button", { name: "Another conversation" }),
  );
  await waitFor(() =>
    expect(screen.queryByText("SQLite [1].")).not.toBeInTheDocument(),
  );
  await user.click(screen.getByRole("button", { name: "Database question" }));
  await screen.findByText("SQLite [1].");
  first.unmount();
  render(<ProjectCloud project="p1" />);
  expect(await screen.findByText("SQLite [1].")).toBeInTheDocument();
  expect(
    vi.mocked(chat.action).mock.calls.filter((call) => call[1] === "send"),
  ).toHaveLength(0);
});
it("failed sends show the error and unlock controls without an automatic paid retry", async () => {
  connected();
  const user = userEvent.setup();
  const normal = vi.mocked(chat.action).getMockImplementation()!;
  vi.mocked(chat.action).mockImplementation(async (p, a, data) => {
    if (a === "send")
      throw new Error("Connection interrupted; inspect the saved chat");
    return normal(p, a, data);
  });
  render(<ProjectCloud project="p1" />);
  const box = screen.getByRole("textbox", { name: "Your question" });
  await waitFor(() => expect(box).toBeEnabled());
  await user.type(box, "Which database?");
  await user.click(screen.getByRole("button", { name: "Send" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Connection interrupted",
  );
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Send" })).toBeEnabled(),
  );
  expect(
    vi.mocked(chat.action).mock.calls.filter((call) => call[1] === "send"),
  ).toHaveLength(1);
});
it("locks repeated confirmations until the backend state has been read back", async () => {
  connected();
  const user = userEvent.setup();
  let finishAction!: () => void;
  let finishRead!: (value: CloudView) => void;
  let verifying = false;
  const pendingAction = new Promise<void>((resolve) => {
    finishAction = resolve;
  });
  const pendingRead = new Promise<CloudView>((resolve) => {
    finishRead = resolve;
  });
  const normal = vi.mocked(cloud.action).getMockImplementation()!;
  vi.mocked(cloud.action).mockImplementation(async (p, a, data) => {
    if (a === "delete_cloud") {
      await pendingAction;
      view.enabled = 0;
      view.paused = 1;
      view.store_id = null;
      view.store_state = "none";
      view.status = "cloud_deleted";
      view.jobs = [];
      view.retainedFiles = 0;
      verifying = true;
      return undefined as never;
    }
    return normal(p, a, data);
  });
  vi.mocked(cloud.view).mockImplementation(async () =>
    verifying ? pendingRead : structuredClone(view),
  );
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeChecked());
  await user.click(toggle);
  const dialog = screen.getByRole("dialog");
  const confirm = within(dialog).getByRole("button", {
    name: "Turn off Cloud sync",
  });
  act(() => {
    fireEvent.click(confirm);
    fireEvent.click(confirm);
  });
  expect(
    vi.mocked(cloud.action).mock.calls.filter((c) => c[1] === "delete_cloud"),
  ).toHaveLength(1);
  expect(toggle).toBeDisabled();
  expect(confirm).toBeDisabled();
  expect(within(dialog).getByRole("status")).toHaveTextContent(
    "checking the saved state",
  );
  await user.click(
    within(dialog).getByRole("button", { name: "Close dialog" }),
  );
  expect(dialog).toBeInTheDocument();
  await act(async () => finishAction());
  expect(confirm).toBeDisabled();
  expect(dialog).toBeInTheDocument();
  await act(async () => finishRead(structuredClone(view)));
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(toggle).not.toBeChecked();
  expect(toggle).toBeEnabled();
  expect(screen.getByRole("status")).toHaveTextContent(
    "Cloud sync is off — confirmed",
  );
});
it("rejects an acknowledged toggle whose backend state did not change", async () => {
  connected();
  const user = userEvent.setup();
  vi.mocked(cloud.action).mockResolvedValue(undefined);
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeChecked());
  await user.click(toggle);
  const dialog = screen.getByRole("dialog");
  await user.click(
    within(dialog).getByRole("button", { name: "Turn off Cloud sync" }),
  );
  expect(await within(dialog).findByRole("alert")).toHaveTextContent(
    "backend did not confirm",
  );
  expect(toggle).toBeChecked();
  expect(dialog).toBeInTheDocument();
  expect(
    within(dialog).getByRole("button", { name: "Turn off Cloud sync" }),
  ).toBeEnabled();
  expect(
    screen.queryByText(/Cloud sync is off — confirmed/),
  ).not.toBeInTheDocument();
});
it("shows verification read failures instead of claiming the toggle completed", async () => {
  connected();
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeChecked());
  await user.click(toggle);
  vi.mocked(cloud.view).mockRejectedValueOnce(
    new Error("Could not read saved cloud status"),
  );
  const dialog = screen.getByRole("dialog");
  await user.click(
    within(dialog).getByRole("button", { name: "Turn off Cloud sync" }),
  );
  expect(await within(dialog).findByRole("alert")).toHaveTextContent(
    "Could not read saved cloud status",
  );
  expect(
    within(dialog).getByRole("button", { name: "Turn off Cloud sync" }),
  ).toBeEnabled();
  expect(
    screen.queryByText(/Cloud sync is off — confirmed/),
  ).not.toBeInTheDocument();
});

it("streams provisional text, then replaces it with the saved answer and filename-only sources", async () => {
  connected();
  chats = [structuredClone(saved)];
  let finish!: (value: Chat) => void;
  let emit!: (text: string) => void;
  const previous = vi.mocked(chat.action).getMockImplementation()!;
  vi.mocked(chat.action).mockImplementation(
    (project, action, payload, preview) => {
      if (action === "send") {
        emit = preview!;
        return new Promise<Chat>((resolve) => {
          finish = resolve;
        }) as never;
      }
      return previous(project, action, payload, preview);
    },
  );
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  await screen.findByText("SQLite [1].");
  await user.type(
    screen.getByRole("textbox", { name: "Your question" }),
    "And the frontend?",
  );
  await user.click(screen.getByRole("button", { name: "Send" }));
  await act(async () => emit("It uses Re"));
  expect(screen.getByText("It uses Re")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Answering…" })).toBeDisabled();
  expect(screen.queryByText("SQLite database")).not.toBeInTheDocument();
  expect(screen.getByText(/Master_Context.md/)).toBeInTheDocument();
  await act(async () => finish(structuredClone(saved)));
  expect(screen.queryByText("It uses Re")).not.toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "Your question" })).toBeEnabled();
  await act(async () => emit("late callback"));
  expect(screen.queryByText("late callback")).not.toBeInTheDocument();
});

it("Enter sends a new chat, Shift+Enter adds a line, and composition Enter does not send", async () => {
  connected();
  const user = userEvent.setup();
  render(<ProjectCloud project="p1" />);
  const input = screen.getByRole("textbox", { name: "Your question" });
  await waitFor(() => expect(input).toBeEnabled());
  await user.type(input, "First line");
  await user.keyboard("{Shift>}{Enter}{/Shift}");
  expect(input).toHaveValue("First line\n");
  fireEvent.keyDown(input, { key: "Enter", isComposing: true });
  expect(
    vi.mocked(chat.action).mock.calls.filter((c) => c[1] === "send"),
  ).toHaveLength(0);
  await user.keyboard("{Enter}");
  await screen.findByText("SQLite [1].");
  expect(
    vi.mocked(chat.action).mock.calls.filter((c) => c[1] === "send"),
  ).toHaveLength(1);
});

it("shows incomplete cloud deletion as cleanup needed and offers a verified retry", async () => {
  connected();
  const user = userEvent.setup();
  let failed = true;
  vi.mocked(cloud.action).mockImplementation(async (_p, action) => {
    if (action === "delete_cloud") {
      if (failed) {
        view.paused = 1;
        view.status = "cleanup_failed";
        view.error = "Network interrupted; cloud deletion incomplete.";
        throw new Error(view.error);
      }
      view.enabled = 0;
      view.paused = 1;
      view.store_id = null;
      view.store_state = "none";
      view.status = "cloud_deleted";
      view.retainedFiles = 0;
      view.jobs = [];
      view.error = null;
    }
    return undefined as never;
  });
  render(<ProjectCloud project="p1" />);
  const toggle = await screen.findByRole("switch", { name: "Cloud sync" });
  await waitFor(() => expect(toggle).toBeChecked());
  await user.click(toggle);
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Turn off Cloud sync",
    }),
  );
  expect(
    await screen.findByText("Cloud deletion incomplete · Retry cleanup"),
  ).toBeInTheDocument();
  expect(
    screen.queryByText("Off · Local version history"),
  ).not.toBeInTheDocument();
  expect(toggle).toBeDisabled();
  failed = false;
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Turn off Cloud sync",
    }),
  );
  await screen.findByText("Off · Local version history");
});
