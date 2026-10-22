import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { ProjectFolder } from "./ProjectFolder";
import { ProjectDetails } from "./ProjectDetails";
import { registry } from "../services/registry";
import type { FolderPlan, Project, Snapshot } from "../types";

vi.mock("../services/registry", () => ({
  registry: {
    pickFolder: vi.fn(),
    previewFolder: vi.fn(),
    reassignFolder: vi.fn(),
    snapshot: vi.fn(),
  },
}));
vi.mock("./ProjectCloud", () => ({
  ProjectCloud: () => <div>Project cloud work</div>,
}));
vi.mock("./TrackingPanel", () => ({
  TrackingPanel: () => <div>Project tracking work</div>,
}));
vi.mock("./FolderTree", () => ({
  FolderTree: () => <div>Folder selection</div>,
}));

const original: Project = {
  id: "project-id",
  name: "Example",
  path: "C:\\old\\Example",
  createdAt: "",
  status: "unavailable",
  lastScan: null,
  scanError: "Folder lost",
  documentCount: 2,
  latestModified: null,
  trackingEnabled: true,
  archiveEnabled: false,
  trackingRevision: 0,
  folderState: "needs_reassignment",
};
const plan: FolderPlan = {
  path: "C:\\new\\Example",
  previousPath: original.path,
  found: 1,
  unchanged: 1,
  changed: 0,
  missing: ["docs/Missing.md"],
  additional: 2,
  token: "review-token",
};
let state: Snapshot;
beforeEach(() => {
  vi.resetAllMocks();
  state = {
    projects: [
      {
        ...original,
        path: plan.path,
        status: "available",
        folderState: "ready",
      },
    ],
    documents: [],
    theme: "system",
  };
  vi.mocked(registry.pickFolder).mockResolvedValue(plan.path);
  vi.mocked(registry.previewFolder).mockResolvedValue(plan);
  vi.mocked(registry.reassignFolder).mockResolvedValue();
  vi.mocked(registry.snapshot).mockImplementation(async () => state);
});
async function review(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "Reassign folder" }));
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  await screen.findByText("Review selected folder");
}

it("keeps the loss reminder after remount and blocks project work while recovery and navigation stay available", async () => {
  const props = {
    project: original,
    documents: [],
    busy: false,
    changed: vi.fn(),
    back: vi.fn(),
    copy: vi.fn(),
    open: vi.fn(),
    rename: vi.fn(),
    remove: vi.fn(),
    refresh: vi.fn(),
  };
  const first = render(<ProjectDetails {...props} />);
  expect(
    screen.getByText("Folder lost — please reassign it"),
  ).toBeInTheDocument();
  expect(screen.queryByText("Project cloud work")).not.toBeInTheDocument();
  expect(screen.queryByText("Project tracking work")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "All projects" })).toBeEnabled();
  first.unmount();
  render(<ProjectDetails {...props} />);
  expect(
    screen.getByText("Folder lost — please reassign it"),
  ).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Reassign folder" })).toBeEnabled();
});

it("reports missing and inaccessible documents after folder recovery", () => {
  const available = {
    ...original,
    status: "available",
    folderState: "ready" as const,
  };
  const doc = {
    id: "source1",
    projectId: original.id,
    documentId: "d1",
    name: "Missing.md",
    relativePath: "Missing.md",
    absolutePath: "C:\\New\\Missing.md",
    kind: "markdown",
    size: 1,
    modifiedAt: null,
    lastObserved: "",
    status: "missing",
  };
  render(
    <ProjectDetails
      project={available}
      documents={[
        doc,
        { ...doc, id: "source2", name: "Unreadable.md", status: "unavailable" },
      ]}
      busy={false}
      changed={vi.fn()}
      back={vi.fn()}
      copy={vi.fn()}
      open={vi.fn()}
      rename={vi.fn()}
      remove={vi.fn()}
      refresh={vi.fn()}
    />,
  );
  expect(
    screen.getByText(/1 files missing · 1 files inaccessible/),
  ).toBeInTheDocument();
  expect(screen.getByText("Project cloud work")).toBeInTheDocument();
  expect(screen.getByText("Project tracking work")).toBeInTheDocument();
});

it("reviews missing files and requires confirmation before preserving the same project identity", async () => {
  const user = userEvent.setup();
  const changed = vi.fn().mockResolvedValue(undefined);
  render(<ProjectFolder project={original} busy={false} changed={changed} />);
  await review(user);
  expect(screen.getByText("docs/Missing.md")).toBeInTheDocument();
  expect(screen.getByText(/1 files missing from/)).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Confirm reassignment" }),
  ).toBeDisabled();
  expect(registry.reassignFolder).not.toHaveBeenCalled();
  await user.click(screen.getByRole("checkbox"));
  await user.click(
    screen.getByRole("button", { name: "Confirm reassignment" }),
  );
  await waitFor(() =>
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
  );
  expect(registry.reassignFolder).toHaveBeenCalledExactlyOnceWith(
    original.id,
    plan.path,
    plan.token,
  );
  expect(changed).toHaveBeenCalledOnce();
});

it("picker cancellation and cancelling the review leave the saved registration untouched", async () => {
  const user = userEvent.setup();
  render(<ProjectFolder project={original} busy={false} changed={vi.fn()} />);
  vi.mocked(registry.pickFolder).mockResolvedValueOnce(null);
  await user.click(screen.getByRole("button", { name: "Reassign folder" }));
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  expect(registry.previewFolder).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  await screen.findByText("Review selected folder");
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  expect(registry.reassignFolder).not.toHaveBeenCalled();
  expect(
    screen.getByText("Folder lost — please reassign it"),
  ).toBeInTheDocument();
});

it("guards duplicate submissions through saved-state verification and retries readback without reapplying", async () => {
  const user = userEvent.setup();
  const changed = vi.fn().mockResolvedValue(undefined);
  let finish: (v: Snapshot) => void = () => {};
  vi.mocked(registry.snapshot).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  render(<ProjectFolder project={original} busy={false} changed={changed} />);
  await review(user);
  await user.click(screen.getByRole("checkbox"));
  await user.dblClick(
    screen.getByRole("button", { name: "Confirm reassignment" }),
  );
  expect(registry.reassignFolder).toHaveBeenCalledOnce();
  expect(
    screen.getByRole("button", { name: "Refresh verification" }),
  ).toBeDisabled();
  await act(async () => {
    finish({ ...state, projects: [original] });
  });
  expect(
    await screen.findByText(/assignment could not be verified/),
  ).toBeInTheDocument();
  expect(changed).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Refresh verification" }),
  );
  await waitFor(() => expect(changed).toHaveBeenCalledOnce());
  expect(registry.reassignFolder).toHaveBeenCalledOnce();
});

it("shows invalid-folder and stale-review failures with a working recovery path", async () => {
  const user = userEvent.setup();
  const changed = vi.fn();
  vi.mocked(registry.previewFolder).mockRejectedValueOnce(
    "This folder overlaps another project.",
  );
  render(<ProjectFolder project={original} busy={false} changed={changed} />);
  await user.click(screen.getByRole("button", { name: "Reassign folder" }));
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  expect(
    await screen.findByText(/overlaps another project/),
  ).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  await screen.findByText("Review selected folder");
  await user.click(screen.getByRole("checkbox"));
  vi.mocked(registry.reassignFolder).mockRejectedValueOnce(
    "The folder changed since review.",
  );
  await user.click(
    screen.getByRole("button", { name: "Confirm reassignment" }),
  );
  expect(await screen.findByText(/changed since review/)).toBeInTheDocument();
  expect(changed).not.toHaveBeenCalled();
  expect(
    screen.getByRole("button", { name: "Choose another folder" }),
  ).toBeEnabled();
});

it("discards an obsolete folder-picker reply after closing the project", async () => {
  const user = userEvent.setup();
  let finish: (p: string) => void = () => {};
  vi.mocked(registry.pickFolder).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const view = render(
    <ProjectFolder project={original} busy={false} changed={vi.fn()} />,
  );
  await user.click(screen.getByRole("button", { name: "Reassign folder" }));
  await user.click(screen.getByRole("button", { name: "Choose folder" }));
  view.unmount();
  await act(async () => finish(plan.path));
  expect(registry.previewFolder).not.toHaveBeenCalled();
});

it("shows persistent missing-file warnings after recovery and keeps all large review lists reachable", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.previewFolder).mockResolvedValue({
    ...plan,
    found: 0,
    missing: Array.from({ length: 205 }, (_, i) => `missing-${i}.md`),
  });
  render(<ProjectFolder project={original} busy={false} changed={vi.fn()} />);
  await review(user);
  const dialog = screen.getByRole("dialog");
  expect(
    within(dialog).getByText(/No known files were found/),
  ).toBeInTheDocument();
  expect(within(dialog).getAllByRole("listitem")).toHaveLength(100);
  await user.click(screen.getByRole("button", { name: "Next missing files" }));
  await user.click(screen.getByRole("button", { name: "Next missing files" }));
  expect(within(dialog).getByText("missing-204.md")).toBeInTheDocument();
});
