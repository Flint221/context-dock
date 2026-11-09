import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { drive, type DriveConnection } from "../drive";
import { open } from "@tauri-apps/plugin-dialog";
import { GoogleDriveConnection } from "./GoogleDriveConnection";

vi.mock("../drive", () => ({
  drive: { connection: vi.fn(), action: vi.fn(), cancel: vi.fn() },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
let state: DriveConnection;
beforeEach(() => {
  vi.resetAllMocks();
  state = {
    configured: false,
    connected: false,
    connecting: false,
    account: null,
    error: null,
  };
  vi.mocked(drive.connection).mockImplementation(async () => ({ ...state }));
  vi.mocked(drive.cancel).mockResolvedValue(undefined);
});
it("imports a native client JSON path without displaying its contents", async () => {
  vi.mocked(open).mockResolvedValue("C:/sample/client.json");
  vi.mocked(drive.action).mockImplementation(async (action) => {
    if (action === "import") state.configured = true;
  });
  const user = userEvent.setup();
  render(<GoogleDriveConnection />);
  await user.click(
    await screen.findByRole("button", { name: "Choose OAuth client JSON" }),
  );
  expect(drive.action).toHaveBeenCalledWith("import", "C:/sample/client.json");
  expect(
    await screen.findByText(/Desktop OAuth client saved/),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Sign in with Google" }),
  ).toBeEnabled();
  expect(screen.queryByText("C:/sample/client.json")).not.toBeInTheDocument();
});
it("does not claim connection success from an unconfirmed backend response", async () => {
  state.configured = true;
  const user = userEvent.setup();
  render(<GoogleDriveConnection />);
  await user.click(
    await screen.findByRole("button", { name: "Sign in with Google" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(/did not confirm/);
  expect(
    screen.queryByText("Google Drive connected and account access verified."),
  ).not.toBeInTheDocument();
});
it("allows cancelling the pending browser flow", async () => {
  state.configured = true;
  let fail: (reason: Error) => void = () => {};
  vi.mocked(drive.action).mockReturnValue(
    new Promise((_resolve, reject) => {
      fail = reject;
    }),
  );
  vi.mocked(drive.cancel).mockImplementation(async () => {
    fail(new Error("Google sign-in cancelled."));
  });
  const user = userEvent.setup();
  render(<GoogleDriveConnection />);
  await user.click(
    await screen.findByRole("button", { name: "Sign in with Google" }),
  );
  await user.click(screen.getByRole("button", { name: "Cancel sign-in" }));
  expect(drive.cancel).toHaveBeenCalledOnce();
  expect(await screen.findByRole("alert")).toHaveTextContent("cancelled");
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Sign in with Google" }),
    ).toBeEnabled(),
  );
});
it("refreshes disconnected state when remote revocation fails", async () => {
  state.configured = true;
  state.connected = true;
  state.account = {
    id: "sample",
    email: "sample@example.test",
    name: "Sample",
  };
  vi.mocked(drive.action).mockImplementation(async () => {
    state.connected = false;
    state.account = null;
    throw new Error(
      "Saved credentials were removed, but Google revocation could not be confirmed.",
    );
  });
  const user = userEvent.setup();
  render(<GoogleDriveConnection />);
  await user.click(
    await screen.findByRole("button", { name: "Disconnect Google Drive" }),
  );
  expect(await screen.findByRole("alert")).toHaveTextContent(
    /revocation could not be confirmed/,
  );
  expect(screen.getByText("Ready to connect")).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Disconnect Google Drive" }),
  ).toBeDisabled();
});
it("can cancel a sign-in started before Settings was reopened", async () => {
  state.configured = true;
  state.connecting = true;
  const user = userEvent.setup();
  const view = render(<GoogleDriveConnection />);
  await user.click(
    await screen.findByRole("button", { name: "Cancel sign-in" }),
  );
  expect(drive.cancel).toHaveBeenCalledOnce();
  await act(async () => view.unmount());
});
