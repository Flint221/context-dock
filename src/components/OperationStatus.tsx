import { CheckCircle2, LoaderCircle } from "lucide-react";
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
  );
}
