import { Component, type ErrorInfo, type ReactNode } from "react";

import { IconAlert, IconRefresh } from "@/components/icons";

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Shown above the error text; names the region that failed. */
  label?: string;
  /** Called when the user asks to retry. */
  onRetry?: () => void;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Contains a render failure to one region of the interface.
 *
 * Without this, a single thrown expression unmounts the whole React tree and the
 * window goes blank — which is exactly how the missing-`notes` bug presented.
 * The user saw nothing at all, with no way to tell whether the app had crashed
 * or was still loading.
 *
 * A boundary is not a substitute for fixing the underlying bug: it turns total
 * failure into a visible, recoverable one. The message is shown verbatim so it
 * can be reported.
 */
export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Keep the component stack in the console; it is the only place it survives.
    console.error(
      `[${this.props.label ?? "ui"}] render failed:`,
      error,
      info.componentStack,
    );
  }

  private reset = (): void => {
    this.setState({ error: null });
    this.props.onRetry?.();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }

    return (
      <div className="empty-state" style={{ height: "100%" }}>
        <span style={{ color: "var(--danger)" }}>
          <IconAlert size={22} />
        </span>
        <div className="empty-state__title">
          {this.props.label
            ? `${this.props.label} could not be displayed`
            : "This view could not be displayed"}
        </div>
        <p
          className="empty-state__body mono"
          style={{ maxWidth: "60ch", overflowWrap: "anywhere" }}
        >
          {error.message}
        </p>
        <button type="button" className="btn" onClick={this.reset}>
          <IconRefresh size={13} />
          Try again
        </button>
      </div>
    );
  }
}
