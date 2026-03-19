import type { Toast } from "../domain/transfer";

export function ToastViewport({ toasts }: { toasts: ReadonlyArray<Toast> }) {
  return (
    <div className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-full max-w-sm flex-col gap-3">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`pointer-events-auto p-4 shadow-sm animate-in slide-in-from-bottom-3 duration-300 ${
            toast.kind === "error"
              ? "bg-ink text-surface"
              : toast.kind === "success"
                ? "bg-brand text-surface"
                : "bg-surface-dim text-ink"
          }`}
        >
          <p className="text-sm font-medium">{toast.message}</p>
        </div>
      ))}
    </div>
  );
}
