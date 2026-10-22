import { createContext, useContext } from "react";
export interface DraftChange {
  dirty: boolean;
  saving: boolean;
  save: () => Promise<boolean>;
  discard: () => void;
}
export interface Guard {
  ready: boolean;
  register: (owner: string, draft: DraftChange) => () => void;
  request: (action: () => void) => void;
}
export const DraftContext = createContext<Guard>({
  ready: true,
  register: () => () => {},
  request: (action) => action(),
});
export function useUnsavedChanges() {
  return useContext(DraftContext);
}
