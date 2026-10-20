/** Unknown durations stay indeterminate; only measured work receives a percentage. */
export function ProgressBar({
  label,
  value,
  detail,
  quiet = false,
}: {
  label: string;
  value?: number;
  detail?: string;
  quiet?: boolean;
}) {
  const measured = value !== undefined && Number.isFinite(value);
  const amount = measured ? Math.max(0, Math.min(100, value)) : undefined;
  return (
    <div className={`progress-meter${quiet ? " progress-meter-quiet" : ""}`}>
      {!quiet && (
        <div className="progress-caption">
          <span>{label}</span>
          {detail && <span>{detail}</span>}
        </div>
      )}
      <div
        className="progress-track"
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={amount}
        aria-valuetext={detail}
        data-indeterminate={!measured || undefined}
      >
        <span
          className="progress-fill"
          style={
            amount === undefined
              ? undefined
              : { transform: `scaleX(${amount / 100})` }
          }
        />
      </div>
    </div>
  );
}
