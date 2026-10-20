import { act, render, screen, waitFor, within } from "@testing-library/react";
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
  vi.mocked(cloud.action).mockImplementation(async (_p, action) => {
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
      view.enabled = 1;
      view.families[0].selected = 1;
    }
    if (action === "start") connected();
    if (action === "disconnect") {
      view.enabled = 0;
      view.paused = 1;
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
  expect(chat.action).toHaveBeenCalledWith("p1", "send", {
    id: "c1",
    question: "Which database?",
  });
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
  expect(cloud.action).not.toHaveBeenCalledWith("p1", "disconnect");
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
