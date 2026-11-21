import { useCallback, useEffect, useState } from "react";

import { ConnectionDialog, emptyConfig } from "@/components/ConnectionDialog";
import {
  BackendGlyph,
  IconEdit,
  IconPlus,
  IconTrash,
} from "@/components/icons";
import { useToast } from "@/components/Toast";
import { useI18n } from "@/i18n/I18nProvider";
import * as ipc from "@/lib/ipc";
import { BACKEND_BLURBS, BACKEND_LABELS } from "@/lib/format";
import type {
  AppMetadata,
  BackendKind,
  ConnectionConfig,
} from "@/types/api";

interface WelcomeProps {
  metadata: AppMetadata | null;
  /** Called once the backend has accepted a connection. */
  onOpened: (sessionId: string) => void;
}

/**
 * Empty state of the workspace.
 *
 * Two jobs: explain what the app talks to, and get a connection open in as few
 * actions as possible. Saved profiles are the primary path; the kind tiles are a
 * shortcut that pre-selects a backend in the form.
 */
export function Welcome({ metadata, onOpened }: WelcomeProps) {
  const toast = useToast();
  const { t } = useI18n();
  const [profiles, setProfiles] = useState<ConnectionConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<ConnectionConfig | null>(null);
  const [connectingId, setConnectingId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setProfiles(await ipc.listProfiles());
    } catch (error) {
      toast.error(
        t("welcome.loadFailed"),
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const openNew = (kind?: BackendKind) => {
    setEditing(kind ? emptyConfig(kind) : null);
    setDialogOpen(true);
  };

  const openSaved = async (profile: ConnectionConfig) => {
    setConnectingId(profile.id);
    try {
      const session = await ipc.connect(profile);
      toast.success("Connected", session.name);
      onOpened(session.session_id);
    } catch (error) {
      toast.error(
        t("welcome.connectFailed"),
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    } finally {
      setConnectingId(null);
    }
  };

  const remove = async (profile: ConnectionConfig) => {
    try {
      await ipc.deleteProfile(profile.id);
      toast.info(t("welcome.removed"), profile.name);
      await refresh();
    } catch (error) {
      toast.error(
        t("welcome.removeFailed"),
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  const kinds = metadata?.backends ?? [];

  return (
    <div className="welcome">
      <div className="welcome__inner">
        <div className="welcome__intro">
          <h1 className="welcome__title">{t("welcome.title")}</h1>
          <p className="welcome__lede">{t("welcome.lede")}</p>
          <div className="welcome__kinds">
            {kinds.map((backend) => (
              <button
                key={backend.kind}
                type="button"
                className="kind-tile"
                onClick={() => openNew(backend.kind)}
                title={BACKEND_BLURBS[backend.kind] ?? ""}
              >
                <span className="kind-tile__glyph">
                  <BackendGlyph kind={backend.kind} size={14} />
                </span>
                {BACKEND_LABELS[backend.kind] ?? backend.label}
              </button>
            ))}
          </div>
        </div>

        <section className="saved">
          <div className="saved__header">
            <h2 className="saved__title">{t("welcome.saved")}</h2>
            <span className="saved__rule" />
            <button
              type="button"
              className="btn btn--sm"
              onClick={() => openNew()}
            >
              <IconPlus size={12} />
              {t("rail.newConnection")}
            </button>
          </div>

          {loading ? (
            <div className="saved__grid">
              <div className="skeleton-row" />
              <div className="skeleton-row" />
            </div>
          ) : profiles.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state__title">{t("welcome.empty.title")}</div>
              <p className="empty-state__body">{t("welcome.empty.body")}</p>
              <button
                type="button"
                className="btn btn--primary"
                onClick={() => openNew()}
              >
                <IconPlus size={13} />
                {t("welcome.empty.cta")}
              </button>
            </div>
          ) : (
            <div className="saved__grid">
              {profiles.map((profile) => (
                <div
                  key={profile.id}
                  className="saved-card"
                  role="button"
                  tabIndex={0}
                  onClick={() => void openSaved(profile)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      void openSaved(profile);
                    }
                  }}
                  aria-label={t("welcome.open")}
                >
                  <div className="saved-card__top">
                    <span className="saved-card__glyph">
                      <BackendGlyph kind={profile.kind} size={15} />
                    </span>
                    <span className="saved-card__name">
                      {profile.name || profile.host}
                    </span>
                    <span className="tag">
                      {BACKEND_LABELS[profile.kind] ?? profile.kind}
                    </span>
                  </div>
                  <div className="saved-card__endpoint">
                    {profile.host}:{profile.port}
                    {profile.tls !== "disabled" ? " · TLS" : ""}
                  </div>
                  <div className="saved-card__actions">
                    <button
                      type="button"
                      className="btn btn--ghost btn--sm"
                      disabled={connectingId === profile.id}
                      onClick={(event) => {
                        event.stopPropagation();
                        void openSaved(profile);
                      }}
                    >
                      {connectingId === profile.id
                        ? t("welcome.opening")
                        : t("welcome.open")}
                    </button>
                    <button
                      type="button"
                      className="btn btn--ghost btn--sm btn--icon"
                      aria-label={t("welcome.edit", { name: profile.name })}
                      onClick={(event) => {
                        event.stopPropagation();
                        setEditing(profile);
                        setDialogOpen(true);
                      }}
                    >
                      <IconEdit size={13} />
                    </button>
                    <button
                      type="button"
                      className="btn btn--danger btn--sm btn--icon"
                      aria-label={t("welcome.remove", { name: profile.name })}
                      onClick={(event) => {
                        event.stopPropagation();
                        void remove(profile);
                      }}
                    >
                      <IconTrash size={13} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      <ConnectionDialog
        open={dialogOpen}
        initial={editing}
        metadata={metadata}
        onClose={() => setDialogOpen(false)}
        onConnected={(sessionId) => {
          onOpened(sessionId);
          void refresh();
        }}
      />
    </div>
  );
}
