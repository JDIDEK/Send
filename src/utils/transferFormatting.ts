import type { TransferProgress } from "../domain/transfer";

export type TransferStatusViewModel = {
  title: string;
  percent: number | null;
  message: string;
  bytesLabel: string;
  speedLabel: string;
  etaLabel: string;
};

function formatBytes(value: number) {
  if (value < 1024) return `${value} o`;
  if (value < 1024 ** 2) return `${(value / 1024).toFixed(1)} Ko`;
  if (value < 1024 ** 3) return `${(value / 1024 ** 2).toFixed(1)} Mo`;
  return `${(value / 1024 ** 3).toFixed(2)} Go`;
}

function formatSpeed(value: number | null) {
  if (!value || value <= 0) return "Calcul...";
  return `${formatBytes(value)}/s`;
}

function formatEta(progress: TransferProgress) {
  if (progress.stage === "finished") return "Transfert terminé.";
  if (progress.stage === "saving") return "Écriture finale sur le disque.";
  if (!progress.etaSeconds || progress.etaSeconds <= 0) return "Bientot";

  const minutes = Math.floor(progress.etaSeconds / 60);
  const seconds = progress.etaSeconds % 60;
  if (minutes === 0) return `${seconds}s restantes`;
  return `${minutes}m ${seconds}s restantes`;
}

export function toTransferStatusViewModel(
  title: string,
  progress: TransferProgress,
): TransferStatusViewModel {
  return {
    title,
    percent: progress.percent,
    message: progress.message ?? "Transfert en cours",
    bytesLabel: progress.totalBytes
      ? `${formatBytes(progress.bytesDone)} / ${formatBytes(progress.totalBytes)}`
      : formatBytes(progress.bytesDone),
    speedLabel: formatSpeed(progress.speedBps),
    etaLabel: formatEta(progress),
  };
}
