import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { registry } from "./services/registry";
import type { Snapshot } from "./types";
vi.mock("./services/registry", () => ({
  registry: {
    snapshot: vi.fn(),
    add: vi.fn(),
    rename: vi.fn(),
    remove: vi.fn(),
    refresh: vi.fn(),
    open: vi.fn(),
    theme: vi.fn(),
    pickFolder: vi.fn(),
  },
}));
let state: Snapshot;
beforeEach(() => {
  vi.clearAllMocks();
  state = { projects: [], documents: [], theme: "system" };
  vi.mocked(registry.snapshot).mockImplementation(async () =>
    structuredClone(state),
  );
});
it("registers a real picker selection and opens its discovered documents", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.pickFolder).mockResolvedValue("C:\\Temporary project");
  vi.mocked(registry.add).mockImplementation(async (name, path) => {
    state.projects.push({
      id: "p1",
      name,
      path,
      createdAt: "",
      status: "available",
      lastScan: null,
      scanError: null,
      documentCount: 1,
      latestModified: null,
    });
    state.documents.push({
      id: "d1",
      projectId: "p1",
      name: "Context.md",
      relativePath: "docs/Context.md",
      absolutePath: path + "\\docs\\Context.md",
      kind: "markdown",
      size: 512,
      modifiedAt: null,
      lastObserved: "",
      status: "available",
    });
    return "p1";
  });
  render(<App />);
  await user.click(await screen.findByText("Add your first project"));
  await user.type(screen.getByLabelText("Project name"), "Cattle Tool");
  await user.click(screen.getByText("Browse"));
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  );
  expect(await screen.findByText("Context.md")).toBeInTheDocument();
  expect(registry.add).toHaveBeenCalledWith(
    "Cattle Tool",
    "C:\\Temporary project",
  );
  await user.click(screen.getByRole("button", { name: "Remove" }));
  expect(screen.getByText(/all its files will remain/)).toBeInTheDocument();
  await user.click(screen.getByText("Cancel"));
  expect(registry.remove).not.toHaveBeenCalled();
});
it("handles cancellation and failed registration without closing the form", async () => {
  const user = userEvent.setup();
  vi.mocked(registry.pickFolder).mockResolvedValue(null);
  render(<App />);
  await user.click(await screen.findByText("Add your first project"));
  await user.type(screen.getByLabelText("Project name"), "Test");
  await user.click(screen.getByText("Browse"));
  expect(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  ).toBeDisabled();
  vi.mocked(registry.pickFolder).mockResolvedValue("C:\\Test");
  await user.click(screen.getByText("Browse"));
  vi.mocked(registry.add).mockRejectedValue(
    "This folder is already registered.",
  );
  await user.click(
    within(screen.getByRole("dialog")).getByRole("button", {
      name: "Add project",
    }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "already registered",
  );
  expect(screen.getByRole("dialog")).toBeInTheDocument();
});
it("persists theme changes and displays the real app version", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Add your first project");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  await user.selectOptions(screen.getByLabelText("Theme"), "dark");
  await waitFor(() => expect(registry.theme).toHaveBeenCalledWith("dark"));
  expect(screen.getByText("Version 0.1.0 · Milestone 1")).toBeInTheDocument();
});
