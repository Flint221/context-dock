import { CheckCircle2, LoaderCircle } from "lucide-react";
import { ProgressBar } from "./ProgressBar";
export function OperationStatus({
  busy,
  message,
  success,
}: {
  busy: boolean;
  message: string;
  success?: string;
}) {
  if (!busy && !success) return null;
  return (
    <div
      className="operation-feedback"
      data-state={busy ? "working" : "complete"}
    >
      <p role="status" className="operation-status" aria-live="polite">
        {busy ? (
          <LoaderCircle
            size={16}
            className="operation-spinner"
            aria-hidden="true"
          />
        ) : (
          <CheckCircle2 size={16} aria-hidden="true" />
        )}
        {busy ? message : success}
      </p>
      <ProgressBar
        quiet
        label={busy ? message : (success ?? message)}
        value={busy ? undefined : 100}
      />
    </div>
  );
}
