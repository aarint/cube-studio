import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { IconClose } from "@/components/icons";

/** Severity of a transient message. */
export type ToastKind = "info" | "success" | "warn" | "error";

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  detail?: string;
}

interface ToastApi {
  push: (toast: Omit<Toast, "id">) => void;
  success: (title: string, detail?: string) => void;
  error: (title: string, detail?: string) => void;
  warn: (title: string, detail?: string) => void;
  info: (title: string, detail?: string) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

/**
 * A single character per severity.
 *
 * Text marks rather than icons: they render identically in every font, stay
 * legible at 10px, and cannot be mistaken for a status icon elsewhere in the
 * UI. Severity is also announced to screen readers via the live region text.
 */
const GLYPH_MARKS: Record<ToastKind, string> = {
  info: "i",
  success: "✓",
  warn: "!",
  error: "!",
};

/** How long a toast stays up. Errors get longer because they carry detail. */
const LIFETIME_MS: Record<ToastKind, number> = {
  info: 4000,
  success: 3000,
  warn: 6000,
  error: 9000,
};

let nextId = 1;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const push = useCallback(
    (input: Omit<Toast, "id">) => {
      const id = nextId;
      nextId += 1;
      setToasts((current) => {
        // Cap the stack: a reconnect storm should not bury the workspace.
        const next = [...current, { ...input, id }];
        return next.slice(-4);
      });
      window.setTimeout(() => dismiss(id), LIFETIME_MS[input.kind]);
    },
    [dismiss],
  );

  const api = useMemo<ToastApi>(
    () => ({
      push,
      success: (title, detail) => push({ kind: "success", title, detail }),
      error: (title, detail) => push({ kind: "error", title, detail }),
      warn: (title, detail) => push({ kind: "warn", title, detail }),
      info: (title, detail) => push({ kind: "info", title, detail }),
    }),
    [push],
  );

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className="toast">
            <span
              className={`toast__glyph toast__glyph--${toast.kind}`}
              aria-hidden="true"
            >
              {GLYPH_MARKS[toast.kind]}
            </span>
            <div className="toast__body">
              <div className="toast__title">
                <span className="sr-only">{toast.kind}: </span>
                {toast.title}
              </div>
              {toast.detail ? (
                <div className="toast__detail">{toast.detail}</div>
              ) : null}
            </div>
            <button
              type="button"
              className="toast__dismiss"
              onClick={() => dismiss(toast.id)}
              aria-label="Dismiss notification"
            >
              <IconClose size={12} />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastApi {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used inside ToastProvider");
  }
  return context;
}
