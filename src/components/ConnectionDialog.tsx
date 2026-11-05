import { useEffect, useMemo, useRef, useState } from "react";

import {
  IconAlert,
  IconCheck,
  IconClose,
  IconTls,
} from "@/components/icons";
import { useToast } from "@/components/Toast";
import * as ipc from "@/lib/ipc";
import { BACKEND_BLURBS, BACKEND_LABELS } from "@/lib/format";
import type {
  AppMetadata,
  BackendKind,
  ConnectionConfig,
  ConnectionInfo,
  RedisMode,
  TlsMode,
} from "@/types/api";

/** A blank profile, used both for "new" and as the shape reference. */
export function emptyConfig(kind: BackendKind = "redis"): ConnectionConfig {
  const ports: Record<BackendKind, number> = {
    redis: 6379,
    memcached: 11211,
    kafka: 9092,
    rabbitmq: 5672,
    nats: 4222,
    mqtt: 1883,
  };
  return {
    id: "",
    name: "",
    kind,
    host: "127.0.0.1",
    port: ports[kind],
    extra_nodes: [],
    username: "",
    password: "",
    tls: "disabled",
    tls_ca_file: null,
    tls_skip_verify: false,
    redis_mode: "standalone",
    redis_db: 0,
    redis_sentinel_master: null,
    kafka_sasl_mechanism: "none",
    kafka_group_id: null,
    vhost: "/",
    client_id: null,
    keep_alive: 30,
    token: null,
    timeout_secs: 10,
    max_payload_bytes: 524288,
    meta: {},
  };
}

/** Which extra fields each backend needs, beyond name/host/port. */
function backendFields(kind: BackendKind) {
  return {
    credentials: kind !== "memcached",
    redis: kind === "redis",
    kafka: kind === "kafka",
    rabbit: kind === "rabbitmq",
    nats: kind === "nats",
    mqtt: kind === "mqtt",
  };
}

interface ConnectionDialogProps {
  open: boolean;
  /** Profile to edit; omit to create a new one. */
  initial?: ConnectionConfig | null;
  /** Called after a successful connect, so the caller can open a workspace. */
  onConnected: (sessionId: string, config: ConnectionConfig) => void;
  onClose: () => void;
  metadata: AppMetadata | null;
}

type Probe =
  | { state: "idle" }
  | { state: "testing" }
  | { state: "ok"; info: ConnectionInfo }
  | { state: "error"; message: string };

export function ConnectionDialog({
  open,
  initial,
  onConnected,
  onClose,
  metadata,
}: ConnectionDialogProps) {
  const toast = useToast();
  const [config, setConfig] = useState<ConnectionConfig>(
    initial ?? emptyConfig(),
  );
  const [remember, setRemember] = useState(true);
  const [probe, setProbe] = useState<Probe>({ state: "idle" });
  const [busy, setBusy] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const nameRef = useRef<HTMLInputElement>(null);

  // Reset the form whenever the dialog is reopened for a different profile.
  useEffect(() => {
    if (open) {
      setConfig(initial ?? emptyConfig());
      setRemember(true);
      setProbe({ state: "idle" });
      setBusy(false);
      setShowAdvanced(Boolean(initial && hasAdvancedValues(initial)));
    }
  }, [open, initial]);

  // Focus the first field so the dialog is usable from the keyboard alone.
  useEffect(() => {
    if (!open) return undefined;
    const id = window.setTimeout(() => nameRef.current?.focus(), 40);
    return () => window.clearTimeout(id);
  }, [open]);

  // Escape closes the dialog.
  useEffect(() => {
    if (!open) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, busy, onClose]);

  const fields = useMemo(() => backendFields(config.kind), [config.kind]);
  const validation = useMemo(() => validate(config), [config]);

  const patch = (changes: Partial<ConnectionConfig>) => {
    setConfig((current) => ({ ...current, ...changes }));
    // Any edit invalidates a previous probe result.
    setProbe({ state: "idle" });
  };

  const handleTest = async () => {
    setProbe({ state: "testing" });
    try {
      const info = await ipc.testConnection(config);
      setProbe({ state: "ok", info });
    } catch (error) {
      setProbe({
        state: "error",
        message: error instanceof ipc.IpcError ? error.message : String(error),
      });
    }
  };

  const handleConnect = async () => {
    if (validation) {
      toast.error("Check the connection settings", validation);
      return;
    }
    setBusy(true);
    try {
      // `save` travels in meta so the backend persists the profile in the same
      // round trip rather than racing a separate save call.
      const meta: Record<string, string> = { ...config.meta };
      if (remember) {
        meta.save = "1";
      } else {
        delete meta.save;
      }

      const session = await ipc.connect({ ...config, meta });
      if (!remember && config.id) {
        // The profile exists but the user unchecked "remember": remove it so the
        // stored password does not linger.
        await ipc.deleteProfile(config.id).catch(() => undefined);
      }
      toast.success(
        "Connected",
        `${session.name} · ${session.endpoints[0] ?? ""}`,
      );
      onConnected(session.session_id, config);
      onClose();
    } catch (error) {
      const message =
        error instanceof ipc.IpcError ? error.message : String(error);
      toast.error("Could not connect", message);
      setProbe({ state: "error", message });
    } finally {
      setBusy(false);
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent) => {
    // Cmd/Ctrl+Enter connects, which is the shortcut people expect in a form.
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      void handleConnect();
    }
  };

  if (!open) {
    return null;
  }

  return (
    <div
      className="modal-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        // Only a click on the backdrop itself dismisses, never a drag that ends
        // up over it.
        if (event.target === event.currentTarget && !busy) {
          onClose();
        }
      }}
    >
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="connection-dialog-title"
        onKeyDown={handleKeyDown}
      >
        <header className="modal__header">
          <h2 className="modal__title" id="connection-dialog-title">
            {initial?.id ? "Edit connection" : "New connection"}
          </h2>
          <button
            type="button"
            className="modal__close"
            onClick={onClose}
            aria-label="Close dialog"
            disabled={busy}
          >
            <IconClose size={14} />
          </button>
        </header>

        <div className="modal__body">
          <section className="form-section">
            <div className="form-section__title">
              Middleware
              <span className="form-section__rule" />
            </div>
            <div className="backend-picker">
              {(metadata?.backends ?? []).map((backend) => (
                <button
                  key={backend.kind}
                  type="button"
                  className={`backend-option ${
                    config.kind === backend.kind ? "backend-option--active" : ""
                  }`}
                  onClick={() =>
                    patch({
                      kind: backend.kind,
                      port: backend.default_port,
                      // Switching family invalidates family-specific state.
                      redis_mode: "standalone",
                      kafka_sasl_mechanism: "none",
                    })
                  }
                  aria-pressed={config.kind === backend.kind}
                >
                  <span className="backend-option__label">
                    {BACKEND_LABELS[backend.kind] ?? backend.label}
                  </span>
                  <span className="backend-option__port">
                    :{backend.default_port}
                  </span>
                </button>
              ))}
            </div>
            <p className="field__hint">{BACKEND_BLURBS[config.kind] ?? ""}</p>
          </section>

          <section className="form-section">
            <div className="form-section__title">
              Endpoint
              <span className="form-section__rule" />
            </div>
            <div className="form-grid">
              <label className="field form-grid__full">
                <span className="field__label">Name</span>
                <input
                  ref={nameRef}
                  className="input"
                  value={config.name}
                  placeholder={`${config.host}:${config.port}`}
                  onChange={(event) => patch({ name: event.target.value })}
                />
              </label>
              <label className="field">
                <span className="field__label">Host</span>
                <input
                  className="input mono"
                  value={config.host}
                  placeholder="127.0.0.1"
                  spellCheck={false}
                  onChange={(event) => patch({ host: event.target.value })}
                />
              </label>
              <label className="field">
                <span className="field__label">Port</span>
                <input
                  className="input mono"
                  type="number"
                  min={1}
                  max={65535}
                  value={config.port}
                  onChange={(event) =>
                    patch({ port: Number(event.target.value) || 0 })
                  }
                />
              </label>
            </div>
            {validation ? <p className="field__error">{validation}</p> : null}
          </section>

          {fields.credentials ? (
            <section className="form-section">
              <div className="form-section__title">
                Authentication
                <span className="form-section__rule" />
              </div>
              <div className="form-grid">
                <label className="field">
                  <span className="field__label">
                    {fields.kafka ? "SASL username" : "Username"}
                  </span>
                  <input
                    className="input"
                    autoComplete="off"
                    value={config.username}
                    onChange={(event) =>
                      patch({ username: event.target.value })
                    }
                  />
                </label>
                <label className="field">
                  <span className="field__label">
                    {fields.kafka ? "SASL password" : "Password"}
                  </span>
                  <input
                    className="input"
                    type="password"
                    autoComplete="off"
                    value={config.password}
                    onChange={(event) =>
                      patch({ password: event.target.value })
                    }
                  />
                </label>
                {fields.nats ? (
                  <label className="field form-grid__full">
                    <span className="field__label">Token</span>
                    <input
                      className="input mono"
                      type="password"
                      autoComplete="off"
                      value={config.token ?? ""}
                      onChange={(event) =>
                        patch({ token: event.target.value || null })
                      }
                    />
                    <span className="field__hint">
                      Used instead of username and password when the server is
                      configured for token auth.
                    </span>
                  </label>
                ) : null}
                {fields.rabbit ? (
                  <label className="field">
                    <span className="field__label">Virtual host</span>
                    <input
                      className="input mono"
                      value={config.vhost}
                      onChange={(event) => patch({ vhost: event.target.value })}
                    />
                  </label>
                ) : null}
                {fields.kafka ? (
                  <label className="field">
                    <span className="field__label">SASL mechanism</span>
                    <select
                      className="select"
                      value={config.kafka_sasl_mechanism}
                      onChange={(event) =>
                        patch({ kafka_sasl_mechanism: event.target.value })
                      }
                    >
                      <option value="none">none (PLAINTEXT)</option>
                      <option value="plain">PLAIN</option>
                      <option value="scram-sha-256">SCRAM-SHA-256</option>
                      <option value="scram-sha-512">SCRAM-SHA-512</option>
                    </select>
                  </label>
                ) : null}
              </div>
            </section>
          ) : null}

          {fields.redis ? (
            <section className="form-section">
              <div className="form-section__title">
                Redis
                <span className="form-section__rule" />
              </div>
              <div className="form-grid">
                <label className="field">
                  <span className="field__label">Deployment</span>
                  <select
                    className="select"
                    value={config.redis_mode}
                    onChange={(event) =>
                      patch({ redis_mode: event.target.value as RedisMode })
                    }
                  >
                    <option value="standalone">Standalone</option>
                    <option value="cluster">Cluster</option>
                    <option value="sentinel">Sentinel</option>
                  </select>
                </label>
                {config.redis_mode === "sentinel" ? (
                  <label className="field">
                    <span className="field__label">Master name</span>
                    <input
                      className="input mono"
                      value={config.redis_sentinel_master ?? ""}
                      placeholder="mymaster"
                      onChange={(event) =>
                        patch({
                          redis_sentinel_master: event.target.value || null,
                        })
                      }
                    />
                  </label>
                ) : (
                  <label className="field">
                    <span className="field__label">Database index</span>
                    <input
                      className="input mono"
                      type="number"
                      min={0}
                      value={config.redis_db}
                      onChange={(event) =>
                        patch({ redis_db: Number(event.target.value) || 0 })
                      }
                    />
                  </label>
                )}
                {config.redis_mode !== "standalone" ? (
                  <label className="field form-grid__full">
                    <span className="field__label">Additional nodes</span>
                    <input
                      className="input mono"
                      value={config.extra_nodes.join(", ")}
                      placeholder="10.0.0.2:6379, 10.0.0.3:6379"
                      onChange={(event) =>
                        patch({
                          extra_nodes: event.target.value
                            .split(",")
                            .map((node) => node.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  </label>
                ) : null}
              </div>
            </section>
          ) : null}

          {fields.mqtt ? (
            <section className="form-section">
              <div className="form-section__title">
                MQTT
                <span className="form-section__rule" />
              </div>
              <div className="form-grid">
                <label className="field">
                  <span className="field__label">Client id</span>
                  <input
                    className="input mono"
                    value={config.client_id ?? ""}
                    placeholder="generated if empty"
                    onChange={(event) =>
                      patch({ client_id: event.target.value || null })
                    }
                  />
                </label>
                <label className="field">
                  <span className="field__label">Keep-alive (s)</span>
                  <input
                    className="input mono"
                    type="number"
                    min={5}
                    max={3600}
                    value={config.keep_alive ?? 30}
                    onChange={(event) =>
                      patch({ keep_alive: Number(event.target.value) || 30 })
                    }
                  />
                </label>
              </div>
            </section>
          ) : null}

          <section className="form-section">
            <div className="form-section__title">
              Transport security
              <span className="form-section__rule" />
            </div>
            <div className="form-grid">
              <label className="field">
                <span className="field__label">
                  <IconTls size={11} /> TLS
                </span>
                <select
                  className="select"
                  value={config.tls}
                  onChange={(event) =>
                    patch({ tls: event.target.value as TlsMode })
                  }
                >
                  <option value="disabled">Disabled</option>
                  <option value="enabled">Enabled</option>
                  <option value="insecure">Enabled, no verification</option>
                </select>
              </label>
              {config.tls !== "disabled" ? (
                <label className="field">
                  <span className="field__label">CA bundle</span>
                  <input
                    className="input mono"
                    value={config.tls_ca_file ?? ""}
                    placeholder="path to PEM (optional)"
                    onChange={(event) =>
                      patch({ tls_ca_file: event.target.value || null })
                    }
                  />
                </label>
              ) : null}
            </div>
            {config.tls === "insecure" ? (
              <div className="notice">
                <IconAlert size={13} className="notice__glyph" />
                <span>
                  Certificates will not be verified. Any host on the network can
                  impersonate this server, so use this only against a throwaway
                  instance.
                </span>
              </div>
            ) : null}
          </section>

          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => setShowAdvanced((value) => !value)}
            style={{ alignSelf: "flex-start" }}
            aria-expanded={showAdvanced}
          >
            {showAdvanced ? "Hide advanced" : "Show advanced"}
          </button>

          {showAdvanced ? (
            <section className="form-section">
              <div className="form-grid">
                <label className="field">
                  <span className="field__label">Timeout (s)</span>
                  <input
                    className="input mono"
                    type="number"
                    min={1}
                    max={600}
                    value={config.timeout_secs ?? 10}
                    onChange={(event) =>
                      patch({ timeout_secs: Number(event.target.value) || 10 })
                    }
                  />
                </label>
                <label className="field">
                  <span className="field__label">Payload limit (KB)</span>
                  <input
                    className="input mono"
                    type="number"
                    min={1}
                    value={Math.round(
                      (config.max_payload_bytes ?? 524288) / 1024,
                    )}
                    onChange={(event) =>
                      patch({
                        max_payload_bytes:
                          (Number(event.target.value) || 512) * 1024,
                      })
                    }
                  />
                  <span className="field__hint">
                    Larger values are truncated before being sent to the window.
                  </span>
                </label>
              </div>
            </section>
          ) : null}

          {probe.state === "ok" ? (
            <div className="probe probe--ok">
              <IconCheck size={14} className="probe__glyph" />
              <div className="probe__body">
                <div className="probe__title">Connection verified</div>
                <div className="probe__detail">
                  {probe.info.server_version
                    ? `server: ${probe.info.server_version}`
                    : "server reachable"}
                </div>
              </div>
            </div>
          ) : null}

          {probe.state === "error" ? (
            <div className="probe probe--error">
              <IconAlert size={14} className="probe__glyph" />
              <div className="probe__body">
                <div className="probe__title">Connection failed</div>
                <div className="probe__detail">{probe.message}</div>
              </div>
            </div>
          ) : null}
        </div>

        <footer className="modal__footer">
          <label className="check">
            <input
              type="checkbox"
              checked={remember}
              onChange={(event) => setRemember(event.target.checked)}
            />
            <span className="check__text">
              <span>Remember this connection</span>
              <span className="check__hint">
                Stored encrypted in your app data directory
              </span>
            </span>
          </label>
          <span className="modal__footer-spacer" />
          <button
            type="button"
            className="btn"
            onClick={() => void handleTest()}
            disabled={probe.state === "testing" || Boolean(validation)}
          >
            {probe.state === "testing" ? "Testing…" : "Test"}
          </button>
          <button
            type="button"
            className="btn btn--primary"
            onClick={() => void handleConnect()}
            disabled={busy || Boolean(validation)}
          >
            {busy ? "Connecting…" : initial?.id ? "Reconnect" : "Connect"}
            <span className="kbd">⌘↵</span>
          </button>
        </footer>
      </div>
    </div>
  );
}

/** Return a blocking validation message, or null when the form is usable. */
export function validate(config: ConnectionConfig): string | null {
  if (!config.host.trim()) {
    return "Host is required.";
  }
  if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) {
    return "Port must be between 1 and 65535.";
  }
  if (
    config.kind === "redis" &&
    config.redis_mode === "sentinel" &&
    !config.redis_sentinel_master?.trim()
  ) {
    return "Sentinel mode needs the master name to resolve.";
  }
  if (
    config.kind === "kafka" &&
    config.kafka_sasl_mechanism !== "none" &&
    !config.username.trim()
  ) {
    return `SASL ${config.kafka_sasl_mechanism.toUpperCase()} needs a username.`;
  }
  return null;
}

/** True when a profile carries settings that live behind "advanced". */
function hasAdvancedValues(config: ConnectionConfig): boolean {
  return Boolean(
    (config.timeout_secs && config.timeout_secs !== 10) ||
      (config.max_payload_bytes && config.max_payload_bytes !== 524288) ||
      config.extra_nodes.length > 0,
  );
}
