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
import { RemoveProject } from "./RemoveProject";
import { cloud, type CloudView } from "../cloud";
import { registry } from "../services/registry";
vi.mock("../cloud", () => ({ cloud: { view: vi.fn(), action: vi.fn() } }));
vi.mock("../services/registry", () => ({
  registry: { remove: vi.fn(), snapshot: vi.fn() },
}));
let view: CloudView;
const project = { id: "p1", name: "Test project" };
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(cloud.action).mockResolvedValue(undefined);
  vi.mocked(registry.remove).mockResolvedValue(undefined);
  view = {
    enabled: 1,
    paused: 0,
    store_id: "vs_test",
    store_name: "ContextDock - Test project",
    store_state: "linked",
    status: "up_to_date",
    retainedFiles: 1,
    jobs: [{ file_id: "file-owned" }],
  } as CloudView;
  vi.mocked(cloud.view).mockImplementation(async () => structuredClone(view));
  vi.mocked(registry.snapshot).mockResolvedValue({
    projects: [],
    documents: [],
    theme: "system",
  });
});
function cleared() {
  view = {
    ...view,
    enabled: 0,
    paused: 1,
    store_id: null,
    store_state: "none",
    status: "cloud_deleted",
    retainedFiles: 0,
    jobs: [],
  };
}
it("defaults to deleting cloud data and guards clicks through cleanup verification before removal", async () => {
  const close = vi.fn(),
    removed = vi.fn();
  let finish!: () => void;
  vi.mocked(cloud.action).mockImplementation(async () => {
    await new Promise<void>((r) => {
      finish = r;
    });
    cleared();
  });
  render(<RemoveProject project={project} close={close} removed={removed} />);
  const confirm = await screen.findByRole("button", { name: "Remove project" });
  await waitFor(() => expect(confirm).toBeEnabled());
  expect(
    screen.getByRole("radio", { name: /Delete the project/ }),
  ).toBeChecked();
  act(() => {
    fireEvent.click(confirm);
    fireEvent.click(confirm);
  });
  expect(cloud.action).toHaveBeenCalledExactlyOnceWith("p1", "delete_cloud", {
    confirm: true,
    confirmation: view.store_name,
    storeId: "vs_test",
  });
  expect(registry.remove).not.toHaveBeenCalled();
  expect(confirm).toBeDisabled();
  expect(screen.getByRole("status")).toHaveTextContent("verifying removal");
  await act(async () => finish());
  await waitFor(() =>
    expect(registry.remove).toHaveBeenCalledExactlyOnceWith("p1", false),
  );
  expect(removed).toHaveBeenCalledOnce();
});
it("retains cloud resources only after an explicit choice", async () => {
  const user = userEvent.setup(),
    removed = vi.fn();
  render(<RemoveProject project={project} close={vi.fn()} removed={removed} />);
  await user.click(
    await screen.findByRole("radio", { name: /Keep cloud data/ }),
  );
  await user.click(
    screen.getByRole("button", { name: "Remove project and keep cloud data" }),
  );
  await waitFor(() => expect(removed).toHaveBeenCalledOnce());
  expect(registry.remove).toHaveBeenCalledExactlyOnceWith("p1", true);
  expect(cloud.action).not.toHaveBeenCalled();
});
it("does not remove the project if cleanup is unverified and cancel performs no deletion", async () => {
  const user = userEvent.setup(),
    close = vi.fn();
  render(<RemoveProject project={project} close={close} removed={vi.fn()} />);
  const confirm = await screen.findByRole("button", { name: "Remove project" });
  await waitFor(() => expect(confirm).toBeEnabled());
  await user.click(confirm);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "did not confirm cloud deletion",
  );
  expect(registry.remove).not.toHaveBeenCalled();
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }),
  );
  expect(close).toHaveBeenCalledOnce();
});
it("retries local removal after successful cleanup without requesting stale cloud deletion again", async () => {
  const user = userEvent.setup();
  vi.mocked(cloud.action).mockImplementation(async () => {
    cleared();
  });
  vi.mocked(registry.remove).mockRejectedValueOnce(
    new Error("Local database busy"),
  );
  const removed = vi.fn();
  render(<RemoveProject project={project} close={vi.fn()} removed={removed} />);
  const confirm = await screen.findByRole("button", { name: "Remove project" });
  await waitFor(() => expect(confirm).toBeEnabled());
  await user.click(confirm);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Local database busy",
  );
  await user.click(confirm);
  await waitFor(() => expect(removed).toHaveBeenCalledOnce());
  expect(cloud.action).toHaveBeenCalledOnce();
});
