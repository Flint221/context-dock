import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
export const desktop = {
  native: () => isTauri(),
  listenClose: async (handler: () => void) => {
    if (!isTauri()) return () => {};
    return getCurrentWindow().onCloseRequested((event) => {
      event.preventDefault();
      handler();
    });
  },
  exit: async () => {
    if (isTauri()) await getCurrentWindow().destroy();
  },
};
