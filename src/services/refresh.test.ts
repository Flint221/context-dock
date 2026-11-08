import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { listen } from "@tauri-apps/api/event";
import { subscribeChanges } from "./refresh";
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
let emit: (payload?: string) => void;
const unlisten = vi.fn();
beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  vi.mocked(listen).mockImplementation(async (_event, callback) => {
    emit = (payload) => callback({ event: "changed", id: 1, payload } as never);
    return unlisten;
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});
it("collapses a burst, filters unrelated projects, and accepts global changes", async () => {
  const read = vi.fn();
  const stop = await subscribeChanges("cloud-changed", read, "p1");
  emit("p2");
  await vi.advanceTimersByTimeAsync(100);
  expect(read).not.toHaveBeenCalled();
  for (let i = 0; i < 20; i++) emit("p1");
  await vi.advanceTimersByTimeAsync(100);
  expect(read).toHaveBeenCalledTimes(1);
  emit();
  await vi.advanceTimersByTimeAsync(100);
  expect(read).toHaveBeenCalledTimes(2);
  stop();
});
it("runs one read at a time and preserves changes received during that read", async () => {
  let resolve!: () => void;
  const read = vi.fn().mockImplementationOnce(
    () =>
      new Promise<void>((done) => {
        resolve = done;
      }),
  );
  const stop = await subscribeChanges("registry-changed", read);
  emit();
  await vi.advanceTimersByTimeAsync(100);
  emit();
  emit();
  await vi.advanceTimersByTimeAsync(500);
  expect(read).toHaveBeenCalledTimes(1);
  resolve();
  await vi.advanceTimersByTimeAsync(100);
  expect(read).toHaveBeenCalledTimes(2);
  stop();
});
it("defers hidden-window reads and refreshes once when visible", async () => {
  const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
  const read = vi.fn();
  const stop = await subscribeChanges("registry-changed", read);
  emit();
  emit();
  await vi.advanceTimersByTimeAsync(1000);
  expect(read).not.toHaveBeenCalled();
  hidden.mockReturnValue(false);
  document.dispatchEvent(new Event("visibilitychange"));
  await vi.advanceTimersByTimeAsync(100);
  expect(read).toHaveBeenCalledTimes(1);
  stop();
});
it("cancels pending and visibility refreshes when the panel unmounts", async () => {
  const read = vi.fn();
  const stop = await subscribeChanges("cloud-changed", read);
  emit();
  stop();
  document.dispatchEvent(new Event("visibilitychange"));
  await vi.advanceTimersByTimeAsync(1000);
  expect(read).not.toHaveBeenCalled();
  expect(unlisten).toHaveBeenCalledTimes(1);
});
