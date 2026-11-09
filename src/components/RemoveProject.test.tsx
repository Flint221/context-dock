import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { RemoveProject } from "./RemoveProject";
import { registry } from "../services/registry";
import { cloud } from "../cloud";
vi.mock("../services/registry", () => ({
  registry: { remove: vi.fn(), snapshot: vi.fn() },
}));
vi.mock("../cloud", () => ({ cloud: { view: vi.fn(), action: vi.fn() } }));
const project = { id: "p1", name: "Test project" };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(registry.remove).mockResolvedValue(undefined);
  vi.mocked(registry.snapshot).mockResolvedValue({
    projects: [],
    documents: [],
    theme: "system",
  });
});
it("removes only the registration and verifies it before closing", async () => {
  const user = userEvent.setup();
  const close = vi.fn(),
    removed = vi.fn();
  render(<RemoveProject project={project} close={close} removed={removed} />);
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(registry.remove).toHaveBeenCalledExactlyOnceWith("p1", true);
  expect(removed).toHaveBeenCalledOnce();
  expect(cloud.action).not.toHaveBeenCalled();
  expect(cloud.view).not.toHaveBeenCalled();
});
it("does not close when removal cannot be verified", async () => {
  const user = userEvent.setup();
  const close = vi.fn();
  vi.mocked(registry.snapshot).mockResolvedValue({
    projects: [{ ...project } as never],
    documents: [],
    theme: "system",
  });
  render(<RemoveProject project={project} close={close} removed={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "could not be verified",
  );
  expect(close).not.toHaveBeenCalled();
});
it("cancels without removing local or remote data", async () => {
  const user = userEvent.setup();
  const close = vi.fn();
  render(<RemoveProject project={project} close={close} removed={vi.fn()} />);
  expect(
    screen.getByText(/Your local files and cloud copies stay in place/),
  ).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  expect(close).toHaveBeenCalledOnce();
  expect(registry.remove).not.toHaveBeenCalled();
  expect(cloud.action).not.toHaveBeenCalled();
});
it("keeps failures reviewable and allows retry without cloud deletion", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.remove).mockRejectedValueOnce(new Error("database busy"));
  const close = vi.fn();
  render(<RemoveProject project={project} close={close} removed={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("database busy");
  await user.click(screen.getByRole("button", { name: "Remove project" }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(cloud.action).not.toHaveBeenCalled();
});
it("guards repeated confirmation while a removal is pending", async () => {
  let finish: () => void = () => {};
  vi.mocked(registry.remove).mockReturnValue(
    new Promise<void>((resolve) => {
      finish = resolve;
    }),
  );
  render(<RemoveProject project={project} close={vi.fn()} removed={vi.fn()} />);
  const button = screen.getByRole("button", { name: "Remove project" });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(registry.remove).toHaveBeenCalledOnce();
  finish();
  await waitFor(() => expect(registry.snapshot).toHaveBeenCalledOnce());
});
