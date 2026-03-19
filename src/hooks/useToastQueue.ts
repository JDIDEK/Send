import { startTransition, useEffect, useRef, useState } from "react";

import type { Toast, ToastKind } from "../domain/transfer";

export function useToastQueue(autoDismissMs = 4000) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timeoutIds = useRef(new Map<number, number>());

  const dismissToast = (id: number) => {
    const timeoutId = timeoutIds.current.get(id);
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      timeoutIds.current.delete(id);
    }

    startTransition(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id));
    });
  };

  const pushToast = (kind: ToastKind, message: string) => {
    const id = Date.now() + Math.floor(Math.random() * 1000);

    startTransition(() => {
      setToasts((current) => [...current, { id, kind, message }]);
    });

    const timeoutId = window.setTimeout(() => {
      dismissToast(id);
    }, autoDismissMs);

    timeoutIds.current.set(id, timeoutId);
  };

  useEffect(() => {
    return () => {
      for (const timeoutId of timeoutIds.current.values()) {
        window.clearTimeout(timeoutId);
      }
      timeoutIds.current.clear();
    };
  }, []);

  return {
    toasts,
    pushToast,
    dismissToast,
  };
}
