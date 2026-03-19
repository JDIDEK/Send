export function ProgressBar({ percent }: { percent: number | null }) {
  const width = `${Math.max(4, Math.min(percent ?? 0, 100))}%`;

  return (
    <div className="h-3 w-full overflow-hidden bg-surface">
      <div
        className="h-full bg-brand transition-[width] duration-300 ease-out"
        style={{ width }}
      />
    </div>
  );
}
