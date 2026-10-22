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
import { useState } from "react";
import { FolderTree } from "./FolderTree";
import { UnsavedChanges } from "../UnsavedChanges";
import { useUnsavedChanges } from "../draftGuard";
import { registry } from "../services/registry";
import { desktop } from "../services/desktop";
import type { PathSelectionView } from "../types";
vi.mock("../services/registry", () => ({
  registry: { paths: vi.fn(), savePaths: vi.fn() },
}));
vi.mock("../services/desktop", () => ({
  desktop: { native: vi.fn(() => false), listenClose: vi.fn(), exit: vi.fn() },
}));
let saved: PathSelectionView;
let closeWindow: (action?: string) => void;
const changed = vi.fn(async () => {});
beforeEach(() => {
  vi.clearAllMocks();
  saved = {
    revision: 0,
    exclusions: [],
    warnings: [],
    cloudPending: 0,
    entries: [
      { path: "Drafts", kind: "folder", status: "available" },
      { path: "Drafts/Nested", kind: "folder", status: "available" },
      { path: "Drafts/Nested/one.md", kind: "file", status: "available" },
      { path: "Keep.md", kind: "file", status: "available" },
    ],
  };
  vi.mocked(desktop.native).mockReturnValue(false);
  vi.mocked(desktop.listenClose).mockImplementation(async (handler) => {
    closeWindow = handler;
    return () => {};
  });
  vi.mocked(desktop.exit).mockResolvedValue(undefined);
  vi.mocked(registry.paths).mockImplementation(async () =>
    structuredClone(saved),
  );
  vi.mocked(registry.savePaths).mockImplementation(
    async (_project, revision, exclusions) => {
      if (revision !== saved.revision) throw new Error("Stale draft");
      saved = {
        ...saved,
        revision: revision + 1,
        exclusions: structuredClone(exclusions),
      };
    },
  );
});
function Contents() {
  const guard = useUnsavedChanges();
  const [left, setLeft] = useState(false);
  return left ? (
    <h1>Other page</h1>
  ) : (
    <>
      <button onClick={() => guard.request(() => setLeft(true))}>
        Leave project
      </button>
      <FolderTree project="p" changed={changed} />
    </>
  );
}
async function ready() {
  const user = userEvent.setup();
  render(
    <UnsavedChanges>
      <Contents />
    </UnsavedChanges>,
  );
  await waitFor(() =>
    expect(
      screen.getByRole("checkbox", { name: "Include Drafts" }),
    ).toBeEnabled(),
  );
  return user;
}
it("shows a new file as excluded until its selection is saved", async () => {
  saved.entries.push({
    path: "Team_Tasks.md",
    kind: "file",
    status: "available",
    new: true,
  });
  saved.exclusions.push({ path: "team_tasks.md", kind: "file" });
  vi.mocked(registry.savePaths).mockImplementation(
    async (_project, revision, exclusions) => {
      saved = {
        ...saved,
        revision: revision + 1,
        exclusions: structuredClone(exclusions),
        entries: saved.entries.map((entry) =>
          entry.path === "Team_Tasks.md" ? { ...entry, new: false } : entry,
        ),
      };
    },
  );
  const user = await ready();
  const checkbox = screen.getByRole("checkbox", {
    name: "Include Team_Tasks.md",
  });
  const row = checkbox.closest(".folder-tree-row") as HTMLElement;
  expect(checkbox).not.toBeChecked();
  expect(within(row).getByText("New")).toBeInTheDocument();
  expect(within(row).getByText("Excluded")).toBeInTheDocument();
  await user.click(checkbox);
  expect(saved.exclusions).toHaveLength(1);
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  await waitFor(() =>
    expect(screen.queryByText("New")).not.toBeInTheDocument(),
  );
  expect(saved.exclusions).toEqual([]);
  expect(
    screen.getByRole("checkbox", { name: "Include Team_Tasks.md" }),
  ).toBeChecked();
});
it("keeps checkbox edits local until Save, then verifies persistent folder exclusion", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
  expect(registry.savePaths).not.toHaveBeenCalled();
  expect(saved.exclusions).toEqual([]);
  await user.click(screen.getByRole("button", { name: "Expand Drafts" }));
  expect(
    screen.getByRole("checkbox", { name: "Include Drafts/Nested" }),
  ).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  await waitFor(() =>
    expect(screen.queryByText("Unsaved changes")).not.toBeInTheDocument(),
  );
  expect(registry.savePaths).toHaveBeenCalledExactlyOnceWith("p", 0, [
    { path: "drafts", kind: "folder" },
  ]);
  expect(saved.exclusions).toEqual([{ path: "drafts", kind: "folder" }]);
  expect(changed).toHaveBeenCalledOnce();
});
it("discards edits without mutating tracking or cloud and selects all from mixed state", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Keep.md" }));
  expect(
    screen.getByRole("checkbox", { name: "Include project folder" }),
  ).toBePartiallyChecked();
  await user.click(
    screen.getByRole("checkbox", { name: "Include project folder" }),
  );
  expect(
    screen.getByRole("checkbox", { name: "Include Keep.md" }),
  ).toBeChecked();
  expect(screen.queryByText("Unsaved changes")).not.toBeInTheDocument();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  await user.click(screen.getByRole("button", { name: "Discard changes" }));
  expect(
    screen.getByRole("checkbox", { name: "Include Drafts" }),
  ).toBeChecked();
  expect(registry.savePaths).not.toHaveBeenCalled();
});
it("prompts on navigation and lets the user keep editing or discard before leaving", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  await user.click(screen.getByRole("button", { name: "Leave project" }));
  expect(
    screen.getByRole("dialog", { name: "Unsaved folder selection" }),
  ).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Keep editing" }));
  expect(
    screen.getByRole("checkbox", { name: "Include Drafts" }),
  ).not.toBeChecked();
  await user.click(screen.getByRole("button", { name: "Leave project" }));
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Discard changes",
    }),
  );
  expect(
    screen.getByRole("heading", { name: "Other page" }),
  ).toBeInTheDocument();
  expect(registry.savePaths).not.toHaveBeenCalled();
});
it("saves and verifies before continuing navigation", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  await user.click(screen.getByRole("button", { name: "Leave project" }));
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  expect(
    await screen.findByRole("heading", { name: "Other page" }),
  ).toBeInTheDocument();
  expect(saved.exclusions).toHaveLength(1);
});
it("intercepts window close, keeps the window open on failed save, and allows verified retry", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  vi.mocked(registry.savePaths).mockRejectedValueOnce(
    new Error("Database busy"),
  );
  act(() => closeWindow());
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  await waitFor(() =>
    expect(
      within(screen.getByRole("dialog")).getByRole("alert"),
    ).toHaveTextContent("could not be saved"),
  );
  expect(desktop.exit).not.toHaveBeenCalled();
  expect(saved.exclusions).toEqual([]);
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  await waitFor(() => expect(desktop.exit).toHaveBeenCalledOnce());
  expect(saved.exclusions).toHaveLength(1);
});
it("routes tray Exit through unsaved protection and preserves the exit action", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  act(() => closeWindow("exit"));
  expect(desktop.exit).not.toHaveBeenCalled();
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Discard changes",
    }),
  );
  await waitFor(() => expect(desktop.exit).toHaveBeenCalledWith("exit"));
  expect(saved.exclusions).toEqual([]);
});
it("does not exit while a save is in flight and can leave after it completes", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  let finish!: () => void;
  vi.mocked(registry.savePaths).mockImplementationOnce(
    async (_p, revision, exclusions) => {
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
      saved = { ...saved, revision: revision + 1, exclusions };
    },
  );
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  await waitFor(() => expect(registry.savePaths).toHaveBeenCalledOnce());
  act(() => closeWindow());
  expect(
    screen.getByRole("button", { name: "Save changes and continue" }),
  ).toBeDisabled();
  expect(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Discard changes",
    }),
  ).toBeDisabled();
  expect(desktop.exit).not.toHaveBeenCalled();
  await act(async () => finish());
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Save changes and continue" }),
    ).toBeEnabled(),
  );
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  await waitFor(() => expect(desktop.exit).toHaveBeenCalledOnce());
});
it("guards browser unload while dirty and accepts saved state after a lost command reply", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  const unload = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(unload);
  expect(unload.defaultPrevented).toBe(true);
  vi.mocked(registry.savePaths).mockImplementationOnce(
    async (_p, revision, exclusions) => {
      saved = { ...saved, revision: revision + 1, exclusions };
      throw new Error("Lost reply");
    },
  );
  const save = screen.getByRole("button", {
    name: "Save changes",
  });
  act(() => {
    fireEvent.click(save);
    fireEvent.click(save);
  });
  await waitFor(() =>
    expect(screen.queryByText("Unsaved changes")).not.toBeInTheDocument(),
  );
  expect(registry.savePaths).toHaveBeenCalledOnce();
  const clean = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(clean);
  expect(clean.defaultPrevented).toBe(false);
});
it("refuses a stale draft without overwriting saved rules", async () => {
  const user = await ready();
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  saved = {
    ...saved,
    revision: 1,
    exclusions: [{ path: "keep.md", kind: "file" }],
  };
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Saved folder choices changed",
  );
  expect(registry.savePaths).not.toHaveBeenCalled();
  expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
});
it("disables edits if the native close guard cannot be installed", async () => {
  vi.mocked(desktop.native).mockReturnValue(true);
  vi.mocked(desktop.listenClose).mockRejectedValueOnce(
    new Error("Listener unavailable"),
  );
  render(
    <UnsavedChanges>
      <Contents />
    </UnsavedChanges>,
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Cannot protect unsaved changes",
  );
  expect(
    await screen.findByRole("checkbox", { name: "Include Drafts" }),
  ).toBeDisabled();
  expect(registry.savePaths).not.toHaveBeenCalled();
});
it("retains a dirty draft when the project folder becomes unavailable", async () => {
  const user = userEvent.setup();
  const page = render(
    <UnsavedChanges>
      <FolderTree project="p" changed={changed} />
    </UnsavedChanges>,
  );
  await waitFor(() =>
    expect(
      screen.getByRole("checkbox", { name: "Include Drafts" }),
    ).toBeEnabled(),
  );
  await user.click(screen.getByRole("checkbox", { name: "Include Drafts" }));
  page.rerender(
    <UnsavedChanges>
      <FolderTree project="p" changed={changed} unavailable />
    </UnsavedChanges>,
  );
  expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
  act(() => closeWindow());
  await user.click(
    screen.getByRole("button", { name: "Save changes and continue" }),
  );
  await waitFor(() =>
    expect(
      within(screen.getByRole("dialog")).getByRole("alert"),
    ).toHaveTextContent("could not be saved"),
  );
  expect(desktop.exit).not.toHaveBeenCalled();
  expect(registry.savePaths).not.toHaveBeenCalled();
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Discard changes",
    }),
  );
  await waitFor(() => expect(desktop.exit).toHaveBeenCalledOnce());
  expect(saved.exclusions).toEqual([]);
});

it("refreshes saved folder coverage after a scan without replacing unsaved checkbox edits", async () => {
  const user = userEvent.setup();
  const { rerender } = render(
    <FolderTree project="p" changed={changed} refreshKey="scan1" />,
  );
  const folder = await screen.findByRole("checkbox", {
    name: "Include Drafts",
  });
  await waitFor(() => expect(folder).toBeEnabled());
  const reads = vi.mocked(registry.paths).mock.calls.length;
  await user.click(folder);
  rerender(<FolderTree project="p" changed={changed} refreshKey="scan2" />);
  expect(folder).not.toBeChecked();
  expect(registry.paths).toHaveBeenCalledTimes(reads);
  await user.click(screen.getByRole("button", { name: "Discard changes" }));
  saved.entries.push({ path: "Added.md", kind: "file", status: "available" });
  rerender(<FolderTree project="p" changed={changed} refreshKey="scan3" />);
  expect(
    await screen.findByRole("checkbox", { name: "Include Added.md" }),
  ).toBeChecked();
});
