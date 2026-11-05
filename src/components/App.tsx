import { useCallback, useEffect, useState } from "react";

import { ConnectionDialog, emptyConfig } from "@/components/ConnectionDialog";
import {
  BackendGlyph,
  IconClose,
  IconPlus,
  IconRefresh,
} from "@/components/icons";
import { useToast } from "@/components/Toast";
import { Welcome } from "@/components/Welcome";
import { Workspace } from "@/components/Workspace";
import * as ipc from "@/lib/ipc";
import { BACKEND_LABELS } from "@/lib/format";
import type { AppMetadata, SessionDescriptor } from "@/types/api";

/**
 * Application shell.
 *
 * Owns the list of open sessions and which one is showing. A session is a live
 * backend connection, so closing one disconnects rather than just hiding it.
 */
export function App() {
  const toast = useToast();
  const [metadata, setMetadata] = useState<AppMetadata | null>(null);
  const [sessions, setSessions] = useState<SessionDescriptor[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [bootError, setBootError] = useState<string | null>(null);

  const active =
    sessions.find((session) => session.session_id === activeId) ?? null;

  // Metadata drives the connection form; without it there is nothing to show.
  useEffect(() => {
    void (async () => {
      try {
        setMetadata(await ipc.appMetadata());
      } catch (error) {
        setBootError(
          error instanceof ipc.IpcError ? error.message : String(error),
        );
      }
    })();
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      const list = await ipc.listSessions();
      setSessions(list);
      setActiveId((current) => {
        if (current && list.some((session) => session.session_id === current)) {
          return current;
        }
        return list[0]?.session_id ?? null;
      });
      return list;
    } catch {
      // Listing sessions is best effort: a failed call should not disturb the
      // workspace the user is already looking at.
      return [];
    }
  }, []);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions]);

  const closeSession = async (sessionId: string) => {
    const session = sessions.find((item) => item.session_id === sessionId);
    const remaining = sessions.filter((item) => item.session_id !== sessionId);
    // Move selection before removing so focus lands somewhere sensible.
    setSessions(remaining);
    setActiveId((current) =>
      current === sessionId ? (remaining[0]?.session_id ?? null) : current,
    );
    try {
      await ipc.disconnect(sessionId);
      toast.info("Disconnected", session?.name ?? "");
    } catch (error) {
      toast.error(
        "Could not disconnect cleanly",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  return (
    <div className="shell">
      <aside className="rail" aria-label="Connections">
        <div className="rail__header">
          <div className="rail__brand">
            <span className="rail__brand-mark" aria-hidden="true">
              ◆
            </span>
            Cube Studio
          </div>
          <span className="rail__version">
            {metadata ? `v${metadata.version}` : ""}
          </span>
        </div>

        <div className="rail__list">
          {sessions.length === 0 ? (
            <p
              className="field__hint"
              style={{ padding: "var(--space-3) var(--space-2)" }}
            >
              No open connections.
            </p>
          ) : (
            <>
              <div className="rail__section-label">Open</div>
              {sessions.map((session) => (
                <div
                  key={session.session_id}
                  className={`rail-item ${
                    session.session_id === activeId ? "rail-item--active" : ""
                  }`}
                  role="button"
                  tabIndex={0}
                  onClick={() => setActiveId(session.session_id)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setActiveId(session.session_id);
                    }
                  }}
                  aria-current={session.session_id === activeId}
                >
                  <span className="rail-item__glyph">
                    <BackendGlyph kind={session.kind} size={14} />
                  </span>
                  <span className="rail-item__body">
                    <span className="rail-item__name" title={session.name}>
                      {session.name}
                    </span>
                    <span className="rail-item__meta">
                      {session.info.server_version
                        ? `${session.info.server_version} · `
                        : ""}
                      {session.endpoints[0] ?? ""}
                    </span>
                  </span>
                  <button
                    type="button"
                    className="rail-item__close"
                    aria-label={`Close ${session.name}`}
                    onClick={(event) => {
                      event.stopPropagation();
                      void closeSession(session.session_id);
                    }}
                  >
                    <IconClose size={11} />
                  </button>
                </div>
              ))}
            </>
          )}
        </div>

        <div className="rail__footer">
          <button
            type="button"
            className="btn btn--primary"
            style={{ width: "100%" }}
            onClick={() => setDialogOpen(true)}
          >
            <IconPlus size={13} />
            New connection
          </button>
        </div>
      </aside>

      <main className="workspace">
        {bootError ? (
          <div className="empty-state" style={{ height: "100%" }}>
            <div className="empty-state__title">The backend did not start</div>
            <p className="empty-state__body">{bootError}</p>
            <button
              type="button"
              className="btn"
              onClick={() => window.location.reload()}
            >
              <IconRefresh size={13} />
              Retry
            </button>
          </div>
        ) : active ? (
          <>
            <header className="workspace__header">
              <div className="workspace__title">
                <span className="workspace__name">{active.name}</span>
                <span className="tag">
                  {BACKEND_LABELS[active.kind] ?? active.kind}
                </span>
              </div>
              <span className="workspace__endpoint">
                {active.endpoints.join(" → ")}
              </span>
              <div className="workspace__actions">
                <button
                  type="button"
                  className="btn btn--ghost btn--sm"
                  onClick={() => void refreshSessions()}
                >
                  <IconRefresh size={12} />
                  Refresh
                </button>
              </div>
            </header>
            <div className="workspace__body">
              <Workspace key={active.session_id} session={active} />
            </div>
          </>
        ) : (
          <div className="workspace__body">
            <Welcome
              metadata={metadata}
              onOpened={() => void refreshSessions()}
            />
          </div>
        )}

        <footer className="statusbar">
          <span className="statusbar__item">
            <span
              className={`statusbar__dot ${active ? "" : "statusbar__dot--idle"}`}
            />
            {active ? "connected" : "idle"}
          </span>
          {active ? (
            <span className="statusbar__item">{active.endpoints[0] ?? ""}</span>
          ) : null}
          <span className="statusbar__spacer" />
          <span className="statusbar__item">
            {sessions.length} session{sessions.length === 1 ? "" : "s"}
          </span>
          {metadata ? (
            <span className="statusbar__item">{metadata.platform}</span>
          ) : null}
        </footer>
      </main>

      <ConnectionDialog
        open={dialogOpen}
        initial={emptyConfig()}
        metadata={metadata}
        onClose={() => setDialogOpen(false)}
        onConnected={() => void refreshSessions()}
      />
    </div>
  );
}
