import { ProgressBar } from "./ProgressBar";

export type TransferStatusCardProps = {
  title: string;
  percent: number | null;
  message: string;
  bytesLabel: string;
  speedLabel: string;
  etaLabel: string;
};

export function TransferStatusCard({
  title,
  percent,
  message,
  bytesLabel,
  speedLabel,
  etaLabel,
}: TransferStatusCardProps) {
  return (
    <div className="w-full space-y-4 p-6 bg-surface-dim animate-in fade-in duration-300">
      <div className="flex items-center justify-between gap-4">
        <h3 className="text-lg font-medium text-ink">{title}</h3>
        <span className="text-sm font-medium text-brand">
          {percent !== null ? `${Math.round(percent)}%` : "En attente"}
        </span>
      </div>

      <ProgressBar percent={percent} />

      <div className="grid grid-cols-1 gap-2 text-sm text-ink-muted md:grid-cols-3">
        <p>{message}</p>
        <p>{bytesLabel}</p>
        <p>{speedLabel}</p>
      </div>

      <p className="text-xs text-ink-muted">{etaLabel}</p>
    </div>
  );
}
