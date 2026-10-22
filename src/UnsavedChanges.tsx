import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { desktop } from "./services/desktop";
import { Modal } from "./components/Modal";
import { DraftContext, type DraftChange } from "./draftGuard";
export function UnsavedChanges({ children }: { children: ReactNode }) {
  const draft = useRef<{ owner: string; change: DraftChange } | null>(null);
  const continuation = useRef<(() => void) | null>(null);
  const [ready, setReady] = useState(!desktop.native());
  const [prompt, setPrompt] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [draftSaving, setDraftSaving] = useState(false);
  const applying = useRef(false);
  const register = useCallback((owner: string, change: DraftChange) => {
    draft.current = { owner, change };
    setDraftSaving(change.saving);
    return () => {
      if (draft.current?.owner === owner) draft.current = null;
    };
  }, []);
  const request = useCallback((action: () => void) => {
    if (
      draft.current &&
      (draft.current.change.dirty || draft.current.change.saving)
    ) {
      if (!continuation.current) {
        continuation.current = action;
        setPrompt(true);
        setError("");
      }
    } else action();
  }, []);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    desktop
      .listenClose((action) =>
        request(() => {
          void desktop.exit(action).catch((e) => setError(String(e)));
        }),
      )
      .then((fn) => {
        if (active) {
          stop = fn;
          setReady(true);
        } else fn();
      })
      .catch((e) => {
        if (active)
          setError(
            `Cannot protect unsaved changes when closing the window. ${String(e)}`,
          );
      });
    const unload = (e: BeforeUnloadEvent) => {
      if (draft.current?.change.dirty || draft.current?.change.saving) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", unload);
    return () => {
      active = false;
      stop?.();
      window.removeEventListener("beforeunload", unload);
    };
  }, [request]);
  const leave = () => {
    const next = continuation.current;
    continuation.current = null;
    draft.current = null;
    setPrompt(false);
    next?.();
  };
  const saveAndLeave = async () => {
    if (applying.current || draft.current?.change.saving) return;
    applying.current = true;
    setBusy(true);
    setError("");
    try {
      if (!draft.current || (await draft.current.change.save())) leave();
      else
        setError(
          "Changes could not be saved and verified. Stay here and resolve the folder-selection error, or discard the draft.",
        );
    } catch (e) {
      setError(String(e));
    } finally {
      applying.current = false;
      setBusy(false);
    }
  };
  const value = useMemo(
    () => ({ ready, register, request }),
    [ready, register, request],
  );
  return (
    <DraftContext.Provider value={value}>
      {error && !prompt && <p role="alert">{error}</p>}
      {children}
      {prompt && (
        <Modal
          title="Unsaved folder selection"
          close={() => {
            if (!busy) {
              continuation.current = null;
              setPrompt(false);
            }
          }}
        >
          <p>
            You have unapplied folder or file checkbox changes. Save them before
            leaving?
          </p>
          <p>
            Saving stops future tracking for excluded paths and removes their
            versions from this project's cloud index. Existing local files and
            history stay safe. Remote removal may need time or a retry. If you
            exit immediately after saving, unfinished index removals resume when
            ContextDock opens again.
          </p>
          {error && <p role="alert">{error}</p>}
          <div className="actions">
            <button
              disabled={busy}
              onClick={() => {
                continuation.current = null;
                setPrompt(false);
              }}
            >
              Keep editing
            </button>
            <button
              disabled={busy || draftSaving}
              onClick={() => {
                draft.current?.change.discard();
                leave();
              }}
            >
              Discard changes
            </button>
            <button
              className="primary"
              disabled={busy || draftSaving}
              onClick={() => void saveAndLeave()}
            >
              {busy ? "Saving…" : "Save changes and continue"}
            </button>
          </div>
        </Modal>
      )}
    </DraftContext.Provider>
  );
}
