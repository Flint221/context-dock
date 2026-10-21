import {
  act,
  render,
  screen,
  waitFor,
  within,
  fireEvent,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import { CurrentProcesses } from "./CurrentProcesses";
import { processes } from "../services/processes";
import type { LocalProcess, ProcessInventory } from "../services/processes";

vi.mock("../services/processes", async (original) => ({
  ...(await original<typeof import("../services/processes")>()),
  processes: {
    inventory: vi.fn(),
    stop: vi.fn(),
    preview: vi.fn(),
    startup: vi.fn(),
    openStartup: vi.fn(),
  },
}));
const app = (
  pid: number,
  name = "node.exe",
  overrides: Partial<LocalProcess> = {},
): LocalProcess => ({
  pid,
  parentPid: 90,
  name,
  path: `C:\\Apps\\${name}`,
  started: `birth-${pid}`,
  startedAt: "2026-10-20T12:00:00Z",
  canStop: true,
  protection: "",
  endpoints: [
    { protocol: "TCP", address: "127.0.0.1", port: 5000 + pid, scope: 0 },
  ],
  ...overrides,
});
let snapshot: ProcessInventory;
beforeEach(() => {
  vi.resetAllMocks();
  snapshot = {
    processes: [
      app(1),
      app(2, "python.exe"),
      app(4, "System", {
        canStop: false,
        protection: "Windows system process protected.",
      }),
    ],
    warnings: [],
    scannedAt: "2026-10-20T12:00:00Z",
  };
  vi.mocked(processes.inventory).mockImplementation(async () =>
    structuredClone(snapshot),
  );
  vi.mocked(processes.stop).mockResolvedValue([
    { pid: 1, stopped: true, message: "Stopped." },
  ]);
  vi.mocked(processes.preview).mockResolvedValue();
  vi.mocked(processes.startup).mockResolvedValue([]);
  vi.mocked(processes.openStartup).mockResolvedValue();
});
afterEach(() => vi.useRealTimers());
async function ready(advanced = true) {
  const view = render(<CurrentProcesses />);
  await screen.findAllByText("node.exe");
  if (advanced)
    await userEvent
      .setup()
      .click(screen.getByRole("button", { name: "Advanced processes" }));
  return view;
}

it("defaults to local preview cards and hides system, UDP, database and unrelated listeners", async () => {
  snapshot.processes.push(
    app(7, "postgres.exe", {
      endpoints: [
        { protocol: "TCP", address: "127.0.0.1", port: 5432, scope: 0 },
      ],
    }),
    app(8, "helper.exe", {
      endpoints: [
        { protocol: "TCP", address: "127.0.0.1", port: 58211, scope: 0 },
      ],
    }),
    app(9, "udp-tool.exe", {
      endpoints: [
        { protocol: "UDP", address: "127.0.0.1", port: 8080, scope: 0 },
      ],
    }),
  );
  await ready(false);
  expect(
    screen.getByRole("button", { name: "Local previews" }),
  ).toHaveAttribute("aria-pressed", "true");
  expect(
    screen.getByRole("button", {
      name: "Preview node.exe port 5001 at 127.0.0.1",
    }),
  ).toBeEnabled();
  expect(screen.getByText("http://127.0.0.1:5001/")).toBeInTheDocument();
  for (const name of ["System", "postgres.exe", "helper.exe", "udp-tool.exe"])
    expect(screen.queryByText(name)).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Protocol")).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "Inspect startup entries" }),
  ).not.toBeInTheDocument();
});
it("keeps simple bulk stop scoped to visible preview apps and retains stop confirmation", async () => {
  snapshot.processes.push(
    app(8, "database.exe", {
      endpoints: [
        { protocol: "TCP", address: "127.0.0.1", port: 5432, scope: 0 },
      ],
    }),
  );
  await ready(false);
  const user = userEvent.setup();
  await user.click(
    screen.getByRole("button", {
      name: "Preview node.exe port 5001 at 127.0.0.1",
    }),
  );
  expect(processes.preview).toHaveBeenCalledWith(
    snapshot.processes[0],
    snapshot.processes[0].endpoints[0],
    false,
  );
  await user.click(
    screen.getByRole("button", { name: "Stop all previews (2)" }),
  );
  const dialog = screen.getByRole("dialog");
  expect(within(dialog).getAllByRole("listitem")).toHaveLength(2);
  expect(within(dialog).queryByText(/database.exe/)).not.toBeInTheDocument();
  expect(processes.stop).not.toHaveBeenCalled();
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(processes.stop).not.toHaveBeenCalled();
});
it("switches reversibly to the full list, clears hidden selections, and defaults back after reopening", async () => {
  const first = await ready(false);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Advanced processes" }));
  expect(screen.getByText("System")).toBeInTheDocument();
  await user.click(
    screen.getByRole("checkbox", { name: "Select node.exe PID 1" }),
  );
  await user.click(screen.getByRole("button", { name: "Local previews" }));
  expect(screen.queryByText("System")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Advanced processes" }));
  expect(
    screen.getByRole("button", { name: "Stop selected (0)" }),
  ).toBeDisabled();
  first.unmount();
  await ready(false);
  expect(
    screen.getByRole("button", { name: "Local previews" }),
  ).toHaveAttribute("aria-pressed", "true");
});

it("keeps protected processes visible but excludes them from bulk actions and cancellation does nothing", async () => {
  const user = userEvent.setup();
  await ready();
  expect(
    screen.getByRole("checkbox", { name: "Select System PID 4" }),
  ).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "Stop all shown (2)" }));
  const dialog = screen.getByRole("dialog");
  expect(within(dialog).getByText("node.exe · PID 1")).toBeInTheDocument();
  expect(within(dialog).getByText("python.exe · PID 2")).toBeInTheDocument();
  expect(within(dialog).queryByText("System · PID 4")).not.toBeInTheDocument();
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(processes.stop).not.toHaveBeenCalled();
});
it("freezes the reviewed targets, ignores double clicks, and reports partial failures without claiming success", async () => {
  const user = userEvent.setup();
  await ready();
  let finish: (
    results: Awaited<ReturnType<typeof processes.stop>>,
  ) => void = () => {};
  vi.mocked(processes.stop).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  await user.click(screen.getByRole("button", { name: "Stop all shown (2)" }));
  const confirm = screen.getByRole("button", {
    name: "Force stop 2 processes",
  });
  fireEvent.click(confirm);
  fireEvent.click(confirm);
  expect(processes.stop).toHaveBeenCalledTimes(1);
  expect(processes.stop).toHaveBeenCalledWith(snapshot.processes.slice(0, 2));
  await act(async () =>
    finish([
      { pid: 1, stopped: true, message: "Stopped." },
      { pid: 2, stopped: false, message: "Access denied. Refresh and retry." },
    ]),
  );
  expect(
    await screen.findByRole("heading", { name: "1 of 2 processes stopped" }),
  ).toBeInTheDocument();
  expect(
    screen.getByText("PID 2: Access denied. Refresh and retry."),
  ).toBeInTheDocument();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});
it("searches ports and scopes Stop all to the current filtered owner list", async () => {
  const user = userEvent.setup();
  await ready();
  await user.type(screen.getByLabelText("Search apps or ports"), "5002");
  expect(screen.queryByText("node.exe")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Stop all shown (1)" }));
  expect(
    within(screen.getByRole("dialog")).getByText("python.exe · PID 2"),
  ).toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Force stop 1 process" }),
  );
  await waitFor(() =>
    expect(processes.stop).toHaveBeenCalledWith([snapshot.processes[1]]),
  );
});
it("clears checkbox selection when unchecked and never silently reselects it", async () => {
  const user = userEvent.setup();
  await ready();
  const checkbox = screen.getByRole("checkbox", {
    name: "Select node.exe PID 1",
  });
  await user.click(checkbox);
  expect(
    screen.getByRole("button", { name: "Stop selected (1)" }),
  ).toBeEnabled();
  await user.click(checkbox);
  await user.click(screen.getByRole("button", { name: "Refresh processes" }));
  await waitFor(() => expect(processes.inventory).toHaveBeenCalledTimes(2));
  expect(checkbox).not.toBeChecked();
  expect(
    screen.getByRole("button", { name: "Stop selected (0)" }),
  ).toBeDisabled();
});
it("invalidates selection when Windows reuses a PID with a different creation time", async () => {
  const user = userEvent.setup();
  await ready();
  await user.click(
    screen.getByRole("checkbox", { name: "Select node.exe PID 1" }),
  );
  snapshot.processes[0].started = "new-process";
  await user.click(screen.getByRole("button", { name: "Refresh processes" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Stop selected (0)" }),
    ).toBeDisabled(),
  );
});
it("previews only local TCP and reverses the HTTPS choice", async () => {
  snapshot.processes[0].endpoints.push(
    { protocol: "UDP", address: "127.0.0.1", port: 6001, scope: 0 },
    { protocol: "TCP", address: "192.168.1.9", port: 7001, scope: 0 },
  );
  const user = userEvent.setup();
  await ready();
  expect(
    screen.queryByRole("button", { name: /Preview node.exe port 6001/ }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: /Preview node.exe port 7001/ }),
  ).not.toBeInTheDocument();
  const preview = screen.getByRole("button", {
    name: "Preview node.exe port 5001 at 127.0.0.1",
  });
  await user.click(screen.getByLabelText("Use HTTPS for previews"));
  await user.click(preview);
  expect(processes.preview).toHaveBeenLastCalledWith(
    snapshot.processes[0],
    snapshot.processes[0].endpoints[0],
    true,
  );
  await user.click(screen.getByLabelText("Use HTTPS for previews"));
  await user.click(preview);
  expect(processes.preview).toHaveBeenLastCalledWith(
    snapshot.processes[0],
    snapshot.processes[0].endpoints[0],
    false,
  );
});
it("shows apps without ports only when requested and filters TCP/UDP and interfaces", async () => {
  snapshot.processes.push(app(5, "Desktop app.exe", { endpoints: [] }));
  snapshot.processes[1].endpoints = [
    { protocol: "UDP", address: "192.168.1.9", port: 9000, scope: 0 },
  ];
  const user = userEvent.setup();
  await ready();
  expect(screen.queryByText("Desktop app.exe")).not.toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText("Show"), "all");
  expect(screen.getByText("Desktop app.exe")).toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText("Protocol"), "UDP");
  expect(screen.getByText("python.exe")).toBeInTheDocument();
  expect(screen.queryByText("node.exe")).not.toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText("Interface"), "local");
  expect(
    screen.getByText("No processes match these filters."),
  ).toBeInTheDocument();
});
it("pauses destructive actions on failed refresh and recovers after a successful retry", async () => {
  const user = userEvent.setup();
  await ready();
  vi.mocked(processes.inventory).mockRejectedValueOnce(
    "Windows inventory failed",
  );
  await user.click(screen.getByRole("button", { name: "Refresh processes" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Actions are paused",
  );
  expect(
    screen.getByRole("button", { name: "Stop node.exe PID 1" }),
  ).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "Refresh processes" }));
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Stop node.exe PID 1" }),
    ).toBeEnabled(),
  );
});
it("keeps a failed native stop review open for retry or cancel", async () => {
  const user = userEvent.setup();
  await ready();
  vi.mocked(processes.stop).mockRejectedValueOnce("Native worker unavailable");
  await user.click(screen.getByRole("button", { name: "Stop node.exe PID 1" }));
  await user.click(
    screen.getByRole("button", { name: "Force stop 1 process" }),
  );
  expect(
    await within(screen.getByRole("dialog")).findByRole("alert"),
  ).toHaveTextContent("Native worker unavailable");
  expect(
    screen.getByRole("button", { name: "Force stop 1 process" }),
  ).toBeEnabled();
});
it("does no recurring work until enabled, and stops polling on opt-out, review, and unmount", async () => {
  const view = await ready();
  vi.useFakeTimers();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(10000);
  });
  expect(processes.inventory).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByLabelText("Refresh every 5 seconds"));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5000);
  });
  expect(processes.inventory).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole("button", { name: "Stop node.exe PID 1" }));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(10000);
  });
  expect(processes.inventory).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  fireEvent.click(screen.getByLabelText("Refresh every 5 seconds"));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(10000);
  });
  expect(processes.inventory).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByLabelText("Refresh every 5 seconds"));
  view.unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(10000);
  });
  expect(processes.inventory).toHaveBeenCalledTimes(2);
});
it("includes every page in explicit bulk review and excludes protected apps", async () => {
  snapshot.processes = Array.from({ length: 30 }, (_, i) => app(i + 100));
  await ready();
  const user = userEvent.setup();
  expect(screen.getByText("30 apps · Page 1 of 2")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Stop all shown (30)" }));
  expect(
    within(screen.getByRole("dialog")).getAllByRole("listitem"),
  ).toHaveLength(30);
});
it("inspects startup information read-only and opens Windows settings only on request", async () => {
  vi.mocked(processes.startup).mockResolvedValue([
    { name: "Preview launcher", source: "Your account · Run · 64-bit" },
  ]);
  const user = userEvent.setup();
  await ready();
  expect(processes.startup).not.toHaveBeenCalled();
  expect(processes.openStartup).not.toHaveBeenCalled();
  await user.click(
    screen.getByRole("button", { name: "Inspect startup entries" }),
  );
  expect(await screen.findByText("Preview launcher")).toBeInTheDocument();
  expect(
    screen.getByText(/Entries may be disabled in Windows/),
  ).toBeInTheDocument();
  await user.click(
    screen.getByRole("button", { name: "Windows Startup apps" }),
  );
  expect(processes.openStartup).toHaveBeenCalledOnce();
  expect(processes.stop).not.toHaveBeenCalled();
});
