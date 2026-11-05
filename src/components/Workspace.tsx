import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  ContainerGlyph,
  IconAlert,
  IconClock,
  IconCopy,
  IconEdit,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconTerminal,
  IconTrash,
} from "@/components/icons";
import { useToast } from "@/components/Toast";
import * as ipc from "@/lib/ipc";
import {
  formatBytes,
  formatTtl,
  looksLikeJson,
  payloadText,
} from "@/lib/format";
import type {
  Container,
  Entry,
  EntryPage,
  SessionDescriptor,
  StatItem,
  StreamMessage,
} from "@/types/api";

interface WorkspaceProps {
  session: SessionDescriptor;
}

/** Number of entries fetched per page. */
const PAGE_SIZE = 200;

type Mode = "value" | "tail" | "console" | "stats";

export function Workspace({ session }: WorkspaceProps) {
  const toast = useToast();
  const caps = session.info.capabilities;

  const [containers, setContainers] = useState<Container[]>([]);
  const [containersLoading, setContainersLoading] = useState(true);
  const [activeContainer, setActiveContainer] = useState<string | null>(
    session.info.default_container ?? null,
  );

  const [page, setPage] = useState<EntryPage | null>(null);
  const [entriesLoading, setEntriesLoading] = useState(false);
  const [search, setSearch] = useState("");

  const [selected, setSelected] = useState<Entry | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const [mode, setMode] = useState<Mode>("value");

  const loadContainers = useCallback(async () => {
    setContainersLoading(true);
    try {
      const list = await ipc.listContainers(session.session_id);
      setContainers(list);
      setActiveContainer((current) => {
        if (current && list.some((container) => container.id === current)) {
          return current;
        }
        return session.info.default_container ?? list[0]?.id ?? null;
      });
    } catch (error) {
      toast.error(
        "Could not list containers",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    } finally {
      setContainersLoading(false);
    }
  }, [session.session_id, session.info.default_container, toast]);

  useEffect(() => {
    void loadContainers();
  }, [loadContainers]);

  const loadEntries = useCallback(
    async (
      container: string,
      options: { append?: boolean; cursor?: string } = {},
    ) => {
      setEntriesLoading(true);
      try {
        const result = await ipc.listEntries(session.session_id, container, {
          cursor: options.cursor ?? null,
          limit: PAGE_SIZE,
          search: search || null,
        });
        setPage((current) =>
          options.append && current
            ? { ...result, entries: [...current.entries, ...result.entries] }
            : result,
        );
      } catch (error) {
        toast.error(
          "Could not list entries",
          error instanceof ipc.IpcError ? error.message : String(error),
        );
        setPage({ entries: [] });
      } finally {
        setEntriesLoading(false);
      }
    },
    [session.session_id, search, toast],
  );

  // Reload when the container or the search term changes. Searching is debounced
  // so typing does not fire a scan per keystroke.
  useEffect(() => {
    if (!activeContainer || !caps.list_entries) {
      setPage(null);
      return undefined;
    }
    const timer = window.setTimeout(
      () => {
        setSelected(null);
        void loadEntries(activeContainer);
      },
      search ? 260 : 0,
    );
    return () => window.clearTimeout(timer);
  }, [activeContainer, search, loadEntries, caps.list_entries]);

  const selectEntry = async (entry: Entry) => {
    if (!activeContainer) return;
    setSelected(entry);
    setDetailLoading(true);
    try {
      setSelected(
        await ipc.readEntry(session.session_id, activeContainer, entry.id),
      );
    } catch (error) {
      toast.error(
        "Could not read entry",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    } finally {
      setDetailLoading(false);
    }
  };

  const refreshEntries = useCallback(() => {
    if (activeContainer) {
      void loadEntries(activeContainer);
    }
  }, [activeContainer, loadEntries]);

  const activeMeta = containers.find(
    (container) => container.id === activeContainer,
  );

  return (
    <div className="panes">
      <aside className="pane" aria-label="Containers">
        <div className="pane__header">
          <span className="pane__title">{containerPaneTitle(session.kind)}</span>
          <span className="pane__count">{containers.length}</span>
          <div className="pane__tools">
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--icon"
              onClick={() => void loadContainers()}
              aria-label="Refresh containers"
              title="Refresh containers"
            >
              <IconRefresh size={13} />
            </button>
          </div>
        </div>
        <div className="pane__body">
          {containersLoading ? (
            <>
              <div className="skeleton-row" />
              <div className="skeleton-row" />
              <div className="skeleton-row" />
            </>
          ) : containers.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state__body">
                {caps.notes[0] ?? "Nothing to browse here."}
              </div>
            </div>
          ) : (
            <div
              className="container-list"
              role="listbox"
              aria-label="Containers"
            >
              {containers.map((container) => (
                <button
                  key={container.id}
                  type="button"
                  role="option"
                  aria-selected={container.id === activeContainer}
                  className={`container-item ${
                    container.id === activeContainer
                      ? "container-item--active"
                      : ""
                  }`}
                  onClick={() => setActiveContainer(container.id)}
                  title={Object.entries(container.meta)
                    .map(([key, value]) => `${key}: ${value}`)
                    .join("\n")}
                >
                  <span className="container-item__glyph">
                    <ContainerGlyph kind={container.kind} size={13} />
                  </span>
                  <span className="container-item__name">{container.name}</span>
                  {container.size !== null && container.size !== undefined ? (
                    <span className="container-item__size">
                      {container.size}
                    </span>
                  ) : null}
                </button>
              ))}
            </div>
          )}
        </div>
        {caps.notes.length > 0 ? (
          <div className="pane__footer">
            <div className="notice notice--info">
              <IconAlert size={12} className="notice__glyph" />
              <span>{caps.notes[0]}</span>
            </div>
          </div>
        ) : null}
      </aside>

      <section className="pane" aria-label="Entries">
        <div className="pane__header">
          <span className="pane__title">{modeTitle(mode)}</span>
          {mode === "value" && page ? (
            <span className="pane__count">
              {page.total ?? page.entries.length}
            </span>
          ) : null}
          <div className="pane__tools">
            {mode === "value" ? (
              <>
                <div className="search">
                  <span className="search__icon">
                    <IconSearch size={11} />
                  </span>
                  <input
                    className="search__input"
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                    placeholder="Filter…"
                    aria-label="Filter entries"
                  />
                </div>
                <button
                  type="button"
                  className="btn btn--ghost btn--sm btn--icon"
                  onClick={refreshEntries}
                  aria-label="Refresh entries"
                  title="Refresh entries"
                >
                  <IconRefresh size={13} />
                </button>
              </>
            ) : null}
          </div>
        </div>

        <ModeTabs mode={mode} onChange={setMode} caps={caps} />

        <div className="pane__body">
          {mode === "value" ? (
            <EntryList
              entries={page?.entries ?? []}
              loading={entriesLoading}
              selectedId={selected?.id ?? null}
              onSelect={(entry) => void selectEntry(entry)}
              onLoadMore={
                page?.next_cursor && activeContainer
                  ? () =>
                      void loadEntries(activeContainer, {
                        append: true,
                        cursor: page.next_cursor ?? undefined,
                      })
                  : undefined
              }
              emptyHint={
                !caps.list_entries
                  ? "This backend cannot enumerate entries. Use the live view."
                  : activeContainer
                    ? "No entries match."
                    : "Select a container first."
              }
            />
          ) : null}

          {mode === "tail" ? (
            <TailPanel
              session={session}
              container={activeContainer}
              onUseEntry={(entry) => {
                setSelected(entry);
                setMode("value");
              }}
            />
          ) : null}

          {mode === "console" ? <ConsolePanel session={session} /> : null}
          {mode === "stats" ? <StatsPanel session={session} /> : null}
        </div>

        {mode === "value" && caps.flush && activeContainer ? (
          <div className="pane__footer">
            <button
              type="button"
              className="btn btn--danger btn--sm"
              onClick={async () => {
                try {
                  const result = await ipc.flushContainer(
                    session.session_id,
                    activeContainer,
                  );
                  toast.success(
                    "Flushed",
                    result.message ?? activeMeta?.name ?? "",
                  );
                  refreshEntries();
                } catch (error) {
                  toast.error(
                    "Could not flush",
                    error instanceof ipc.IpcError
                      ? error.message
                      : String(error),
                  );
                }
              }}
            >
              <IconTrash size={12} />
              Flush {activeMeta?.name ?? "container"}
            </button>
          </div>
        ) : null}
      </section>

      <section className="pane pane--detail" aria-label="Detail">
        <DetailPane
          session={session}
          container={activeContainer}
          entry={selected}
          loading={detailLoading}
          onChanged={refreshEntries}
        />
      </section>
    </div>
  );
}

function containerPaneTitle(kind: string): string {
  switch (kind) {
    case "redis":
      return "Databases";
    case "kafka":
      return "Topics";
    case "rabbitmq":
      return "Queues";
    case "nats":
      return "Streams";
    case "mqtt":
      return "Filters";
    default:
      return "Server";
  }
}

function modeTitle(mode: Mode): string {
  switch (mode) {
    case "tail":
      return "Live";
    case "console":
      return "Console";
    case "stats":
      return "Stats";
    default:
      return "Entries";
  }
}

// ---- tabs --------------------------------------------------------------

function ModeTabs({
  mode,
  onChange,
  caps,
}: {
  mode: Mode;
  onChange: (mode: Mode) => void;
  caps: SessionDescriptor["info"]["capabilities"];
}) {
  const tabs: Array<{ id: Mode; label: string }> = [];
  if (caps.list_entries || caps.containers) {
    tabs.push({ id: "value", label: "Browse" });
  }
  if (caps.tail) {
    tabs.push({ id: "tail", label: "Live" });
  }
  if (caps.console) {
    tabs.push({ id: "console", label: "Console" });
  }
  if (caps.stats) {
    tabs.push({ id: "stats", label: "Stats" });
  }

  if (tabs.length <= 1) {
    return null;
  }

  return (
    <div className="detail__tabs" role="tablist" aria-label="View">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={mode === tab.id}
          className={`tab ${mode === tab.id ? "tab--active" : ""}`}
          onClick={() => onChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

// ---- entry list --------------------------------------------------------

function EntryList({
  entries,
  loading,
  selectedId,
  onSelect,
  onLoadMore,
  emptyHint,
}: {
  entries: Entry[];
  loading: boolean;
  selectedId: string | null;
  onSelect: (entry: Entry) => void;
  onLoadMore?: () => void;
  emptyHint: string;
}) {
  if (loading && entries.length === 0) {
    return (
      <>
        <div className="skeleton-row" />
        <div className="skeleton-row" />
        <div className="skeleton-row" />
        <div className="skeleton-row" />
      </>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-state__body">{emptyHint}</div>
      </div>
    );
  }

  return (
    <div className="entry-list" role="listbox" aria-label="Entries">
      {entries.map((entry) => (
        <button
          key={entry.id}
          type="button"
          role="option"
          aria-selected={entry.id === selectedId}
          className={`entry-item ${
            entry.id === selectedId ? "entry-item--active" : ""
          }`}
          onClick={() => onSelect(entry)}
        >
          <span className="entry-item__label">{entry.label}</span>
          <span className="entry-item__aside">
            {entry.ttl !== null && entry.ttl !== undefined ? (
              <span title={`TTL ${formatTtl(entry.ttl)}`}>
                <IconClock size={10} /> {formatTtl(entry.ttl)}
              </span>
            ) : null}
            {entry.size !== null && entry.size !== undefined ? (
              <span>{formatBytes(entry.size)}</span>
            ) : null}
          </span>
          <span className="entry-item__sub">
            <span className={`kind-chip kind-chip--${entry.kind}`}>
              {entry.kind}
            </span>
            {entry.meta.preview ? ` ${entry.meta.preview}` : ""}
          </span>
        </button>
      ))}
      {onLoadMore ? (
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          style={{ margin: "var(--space-2) var(--space-3)" }}
          onClick={onLoadMore}
          disabled={loading}
        >
          {loading ? "Loading…" : "Load more"}
        </button>
      ) : null}
    </div>
  );
}

// ---- detail ------------------------------------------------------------

function DetailPane({
  session,
  container,
  entry,
  loading,
  onChanged,
}: {
  session: SessionDescriptor;
  container: string | null;
  entry: Entry | null;
  loading: boolean;
  onChanged: () => void;
}) {
  const toast = useToast();
  const caps = session.info.capabilities;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [view, setView] = useState<"text" | "raw">("text");

  useEffect(() => {
    setEditing(false);
    setView("text");
  }, [entry?.id, container]);

  if (!entry) {
    return (
      <div className="detail">
        <div className="empty-state">
          <div className="empty-state__title">Nothing selected</div>
          <p className="empty-state__body">
            Pick an entry to inspect its payload.
          </p>
          {caps.publish && container ? (
            <PublishButton session={session} container={container} />
          ) : null}
        </div>
      </div>
    );
  }

  const payload = entry.payload;
  const text = payload ? payloadText(payload) : "";
  const isJson = payload?.json !== undefined && payload?.json !== null;

  const startEdit = () => {
    setDraft(isJson ? JSON.stringify(payload?.json, null, 2) : text);
    setEditing(true);
  };

  const save = async () => {
    if (!container) return;
    // Reject broken JSON before it reaches the store: writing a half-edited
    // object is the most common way to lose data here.
    if (looksLikeJson(draft) === false && draft.trim().startsWith("{")) {
      toast.error("Invalid JSON", "Fix the brackets before saving.");
      return;
    }
    try {
      await ipc.writeEntry(
        session.session_id,
        container,
        entry.id,
        { text: draft },
        {
          entry_kind: entry.kind === "unknown" ? null : entry.kind,
          ttl: entry.ttl && entry.ttl > 0 ? entry.ttl : null,
        },
      );
      toast.success("Saved", entry.label);
      setEditing(false);
      onChanged();
    } catch (error) {
      toast.error(
        "Could not save",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  const remove = async () => {
    if (!container) return;
    try {
      await ipc.deleteEntries(session.session_id, container, [entry.id]);
      toast.success("Deleted", entry.label);
      onChanged();
    } catch (error) {
      toast.error(
        "Could not delete",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      toast.info("Copied to clipboard");
    } catch {
      toast.warn("Clipboard unavailable");
    }
  };

  return (
    <div className="detail">
      <div className="detail__header">
        <span className={`kind-chip kind-chip--${entry.kind}`}>
          {entry.kind}
        </span>
        <span className="detail__key" title={entry.label}>
          {entry.label}
        </span>
        <div className="detail__actions">
          <button
            type="button"
            className="btn btn--ghost btn--sm btn--icon"
            onClick={() => void copy()}
            aria-label="Copy payload"
            title="Copy payload"
          >
            <IconCopy size={13} />
          </button>
          {caps.write_entries ? (
            <button
              type="button"
              className="btn btn--ghost btn--sm btn--icon"
              onClick={startEdit}
              aria-label="Edit payload"
              title="Edit payload"
            >
              <IconEdit size={13} />
            </button>
          ) : null}
          {caps.delete_entries ? (
            <button
              type="button"
              className="btn btn--danger btn--sm btn--icon"
              onClick={() => void remove()}
              aria-label="Delete entry"
              title="Delete entry"
            >
              <IconTrash size={13} />
            </button>
          ) : null}
          {caps.publish && container ? (
            <PublishButton session={session} container={container} />
          ) : null}
        </div>
      </div>

      <div className="detail__meta">
        {entry.size !== null && entry.size !== undefined ? (
          <span className="detail__meta-pair">
            size
            <span className="detail__meta-value">
              {formatBytes(entry.size)}
            </span>
          </span>
        ) : null}
        {entry.ttl !== null && entry.ttl !== undefined ? (
          <span className="detail__meta-pair">
            ttl
            <span className="detail__meta-value">{formatTtl(entry.ttl)}</span>
          </span>
        ) : null}
        {Object.entries(entry.meta)
          .filter(([key]) => key !== "preview")
          .slice(0, 6)
          .map(([key, value]) => (
            <span className="detail__meta-pair" key={key}>
              {key}
              <span className="detail__meta-value">{value}</span>
            </span>
          ))}
      </div>

      {editing ? (
        <Editor
          draft={draft}
          onChange={setDraft}
          onSave={() => void save()}
          onCancel={() => setEditing(false)}
        />
      ) : (
        <div className="detail__body">
          {isJson ? (
            <div className="detail__tabs" role="tablist" aria-label="Format">
              <button
                type="button"
                role="tab"
                aria-selected={view === "text"}
                className={`tab ${view === "text" ? "tab--active" : ""}`}
                onClick={() => setView("text")}
              >
                JSON
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={view === "raw"}
                className={`tab ${view === "raw" ? "tab--active" : ""}`}
                onClick={() => setView("raw")}
              >
                Raw
              </button>
            </div>
          ) : null}
          <pre
            className={`payload ${payload?.binary ? "payload--binary" : ""}`}
            aria-busy={loading}
          >
            {loading
              ? "Loading…"
              : view === "text"
                ? text
                : (payload?.text ?? text)}
            {payload?.truncated ? (
              <span className="payload__truncated">
                Showing {formatBytes(payload.size)} of a truncated value. Raise
                the payload limit in the connection settings to see all of it.
              </span>
            ) : null}
          </pre>
        </div>
      )}
    </div>
  );
}

function Editor({
  draft,
  onChange,
  onSave,
  onCancel,
}: {
  draft: string;
  onChange: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  const jsonInvalid = useMemo(() => {
    const trimmed = draft.trim();
    if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
      return false;
    }
    try {
      JSON.parse(trimmed);
      return false;
    } catch {
      return true;
    }
  }, [draft]);

  return (
    <div className="editor">
      <textarea
        className={`editor__input ${jsonInvalid ? "editor__input--invalid" : ""}`}
        value={draft}
        onChange={(event) => onChange(event.target.value)}
        spellCheck={false}
        autoFocus
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            onCancel();
          }
          if ((event.metaKey || event.ctrlKey) && event.key === "s") {
            event.preventDefault();
            onSave();
          }
        }}
        placeholder="Value"
      />
      <div className="editor__bar">
        <span
          className={`editor__hint ${jsonInvalid ? "editor__hint--invalid" : ""}`}
        >
          {jsonInvalid ? "Invalid JSON" : "⌘S to save · Esc to cancel"}
        </span>
        <button type="button" className="btn btn--sm" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="btn btn--primary btn--sm"
          onClick={onSave}
          disabled={jsonInvalid}
        >
          Save
        </button>
      </div>
    </div>
  );
}

// ---- publish -----------------------------------------------------------

function PublishButton({
  session,
  container,
}: {
  session: SessionDescriptor;
  container: string;
}) {
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [body, setBody] = useState("");
  const [routingKey, setRoutingKey] = useState("");
  const [retained, setRetained] = useState(false);
  const [qos, setQos] = useState(1);
  const [partition, setPartition] = useState(0);

  if (!session.info.capabilities.publish) {
    return null;
  }

  const send = async () => {
    try {
      const result = await ipc.publishMessage(
        session.session_id,
        container,
        { text: body },
        {
          routing_key: routingKey || null,
          retained: session.kind === "mqtt" ? retained : null,
          qos: session.kind === "mqtt" ? qos : null,
          partition: session.kind === "kafka" ? partition : null,
        },
      );
      toast.success("Published", result.message ?? "");
      setOpen(false);
      setBody("");
    } catch (error) {
      toast.error(
        "Publish failed",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  return (
    <>
      <button
        type="button"
        className="btn btn--sm"
        onClick={() => setOpen(true)}
      >
        <IconPlus size={12} />
        Publish
      </button>
      {open ? (
        <div
          className="modal-backdrop"
          role="presentation"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setOpen(false);
          }}
        >
          <div className="modal" role="dialog" aria-modal="true">
            <header className="modal__header">
              <h2 className="modal__title">Publish message</h2>
            </header>
            <div className="modal__body">
              {session.kind === "kafka" ? (
                <label className="field">
                  <span className="field__label">Partition</span>
                  <input
                    className="input mono"
                    type="number"
                    min={0}
                    value={partition}
                    onChange={(event) =>
                      setPartition(Number(event.target.value) || 0)
                    }
                  />
                </label>
              ) : null}
              {session.kind === "rabbitmq" ||
              session.kind === "nats" ||
              session.kind === "mqtt" ? (
                <div className="form-grid">
                  <label className="field">
                    <span className="field__label">
                      {session.kind === "nats"
                        ? "Subject"
                        : session.kind === "mqtt"
                          ? "Topic"
                          : "Routing key"}
                    </span>
                    <input
                      className="input mono"
                      value={routingKey}
                      onChange={(event) => setRoutingKey(event.target.value)}
                      placeholder={
                        session.kind === "mqtt"
                          ? "sensors/temp"
                          : session.kind === "nats"
                            ? "orders.created"
                            : "orders"
                      }
                    />
                  </label>
                  {session.kind === "mqtt" ? (
                    <label className="field">
                      <span className="field__label">QoS</span>
                      <select
                        className="select"
                        value={qos}
                        onChange={(event) => setQos(Number(event.target.value))}
                      >
                        <option value={0}>0 — at most once</option>
                        <option value={1}>1 — at least once</option>
                        <option value={2}>2 — exactly once</option>
                      </select>
                    </label>
                  ) : null}
                </div>
              ) : null}
              {session.kind === "mqtt" ? (
                <label className="check">
                  <input
                    type="checkbox"
                    checked={retained}
                    onChange={(event) => setRetained(event.target.checked)}
                  />
                  <span className="check__text">
                    <span>Retain on the broker</span>
                    <span className="check__hint">
                      New subscribers receive this message immediately
                    </span>
                  </span>
                </label>
              ) : null}
              <label className="field">
                <span className="field__label">Payload</span>
                <textarea
                  className="textarea"
                  rows={8}
                  value={body}
                  onChange={(event) => setBody(event.target.value)}
                  spellCheck={false}
                  placeholder='{"hello":"world"}'
                />
              </label>
            </div>
            <footer className="modal__footer">
              <span className="modal__footer-spacer" />
              <button
                type="button"
                className="btn"
                onClick={() => setOpen(false)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn--primary"
                onClick={() => void send()}
              >
                Publish
              </button>
            </footer>
          </div>
        </div>
      ) : null}
    </>
  );
}

// ---- live tail ---------------------------------------------------------

/** Sensible starting point per backend, so the field is never empty. */
function defaultTailTarget(kind: string, container: string | null): string {
  switch (kind) {
    case "redis":
      return "channel:*";
    case "nats":
      return "subject:>";
    case "mqtt":
      return container ?? "topic:#";
    default:
      return container ?? "";
  }
}

function TailPanel({
  session,
  container,
  onUseEntry,
}: {
  session: SessionDescriptor;
  container: string | null;
  onUseEntry: (entry: Entry) => void;
}) {
  const toast = useToast();
  const [target, setTarget] = useState(() =>
    defaultTailTarget(session.kind, container),
  );
  const [subscriptionId, setSubscriptionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<StreamMessage[]>([]);
  const [paused, setPaused] = useState(false);
  const [pausedCount, setPausedCount] = useState(0);
  // Bounded backlog: an idle tab must not grow without limit.
  const buffer = useRef<StreamMessage[]>([]);

  const running = subscriptionId !== null;

  useEffect(() => {
    setTarget((current) =>
      current ? current : defaultTailTarget(session.kind, container),
    );
  }, [container, session.kind]);

  useEffect(() => {
    if (!subscriptionId) return undefined;
    const unlisten = ipc.onStreamMessage((message) => {
      if (message.subscription_id !== subscriptionId) return;
      if (paused) {
        buffer.current = [...buffer.current.slice(-499), message];
        setPausedCount(buffer.current.length);
        return;
      }
      setMessages((current) => [...current.slice(-999), message]);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [subscriptionId, paused]);

  useEffect(() => {
    if (!subscriptionId) return undefined;
    const unlisten = ipc.onStreamClosed((event) => {
      if (event.subscription_id !== subscriptionId) return;
      setSubscriptionId(null);
      toast.info(
        "Stream ended",
        `${event.messages} message${event.messages === 1 ? "" : "s"} received`,
      );
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [subscriptionId, toast]);

  // Always stop the backend subscription when the panel unmounts.
  useEffect(
    () => () => {
      if (subscriptionId) {
        void ipc.stopTail(subscriptionId).catch(() => undefined);
      }
    },
    [subscriptionId],
  );

  const start = async () => {
    try {
      const id = await ipc.startTail(session.session_id, target);
      setSubscriptionId(id);
      setMessages([]);
      buffer.current = [];
      setPausedCount(0);
      setPaused(false);
    } catch (error) {
      toast.error(
        "Could not subscribe",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
    }
  };

  const stop = async () => {
    if (subscriptionId) {
      await ipc.stopTail(subscriptionId).catch(() => undefined);
    }
    setSubscriptionId(null);
  };

  const resume = () => {
    setPaused(false);
    if (buffer.current.length > 0) {
      setMessages((current) => [...current, ...buffer.current]);
      buffer.current = [];
      setPausedCount(0);
    }
  };

  return (
    <div className="tail">
      <div className="tail__bar">
        <input
          className="input mono"
          style={{ height: 26, fontSize: 12 }}
          value={target}
          onChange={(event) => setTarget(event.target.value)}
          disabled={running}
          aria-label="Subscription target"
          placeholder={defaultTailTarget(session.kind, container)}
        />
        {running ? (
          <>
            <button
              type="button"
              className="btn btn--sm"
              onClick={() => (paused ? resume() : setPaused(true))}
            >
              {paused ? `Resume (${pausedCount})` : "Pause"}
            </button>
            <button
              type="button"
              className="btn btn--sm"
              onClick={() => void stop()}
            >
              Stop
            </button>
          </>
        ) : (
          <button
            type="button"
            className="btn btn--primary btn--sm"
            onClick={() => void start()}
            disabled={!target.trim()}
          >
            Subscribe
          </button>
        )}
        <button
          type="button"
          className="btn btn--ghost btn--sm btn--icon"
          onClick={() => setMessages([])}
          aria-label="Clear messages"
          title="Clear messages"
        >
          <IconTrash size={12} />
        </button>
      </div>

      {running ? (
        <div className="tail__bar" style={{ borderTop: 0 }}>
          <span className="tail__status">
            <span className="tail__pulse" aria-hidden="true" />
            live · {messages.length} received
          </span>
        </div>
      ) : null}

      <div className="tail__log" role="log" aria-live="polite">
        {messages.length === 0 ? (
          <div className="tail__empty">
            {running
              ? "Waiting for messages on this subscription…"
              : "Subscribe to watch messages arrive in real time."}
          </div>
        ) : (
          messages.map((message, index) => (
            <div
              className="tail__row"
              key={`${message.timestamp}-${index}`}
              onClick={() =>
                onUseEntry({
                  id: message.position ?? message.source,
                  label: message.source,
                  kind: "message",
                  meta: { preview: message.payload.text ?? "" },
                  payload: message.payload,
                })
              }
            >
              <span className="tail__time">
                {new Date(message.timestamp).toLocaleTimeString([], {
                  hour12: false,
                })}
              </span>
              <span className="tail__msg">
                <span className="tail__source">{message.source}</span>{" "}
                <span className="tail__payload">
                  {message.payload.text ?? "(binary)"}
                </span>
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ---- console -----------------------------------------------------------

function ConsolePanel({ session }: { session: SessionDescriptor }) {
  const [history, setHistory] = useState<
    Array<{ command: string; output: string; error: boolean; ms: number }>
  >([]);
  const [command, setCommand] = useState("");
  const [busy, setBusy] = useState(false);
  const outputRef = useRef<HTMLDivElement>(null);

  const placeholder =
    session.kind === "redis"
      ? "GET some:key"
      : session.kind === "nats"
        ? "subject  payload"
        : session.kind === "mqtt"
          ? "topic  payload"
          : session.kind === "rabbitmq"
            ? "queue_declare orders"
            : "get some:key";

  const run = async () => {
    const trimmed = command.trim();
    if (!trimmed) return;
    setBusy(true);
    setCommand("");
    try {
      const result = await ipc.runConsole(session.session_id, trimmed);
      setHistory((current) => [
        ...current,
        {
          command: trimmed,
          output: result.output,
          error: result.error,
          ms: result.elapsed_ms,
        },
      ]);
    } catch (error) {
      setHistory((current) => [
        ...current,
        {
          command: trimmed,
          output: error instanceof ipc.IpcError ? error.message : String(error),
          error: true,
          ms: 0,
        },
      ]);
    } finally {
      setBusy(false);
      window.setTimeout(() => {
        outputRef.current?.scrollTo({ top: outputRef.current.scrollHeight });
      }, 0);
    }
  };

  return (
    <div className="console">
      <div className="console__output" ref={outputRef}>
        {history.length === 0 ? (
          <div className="tail__empty">
            Run a command against this session.
          </div>
        ) : (
          history.map((item, index) => (
            <div className="console__entry" key={index}>
              <div>
                <span className="console__prompt">›&nbsp;</span>
                <span className="console__command">{item.command}</span>
                <span className="console__timing">{item.ms} ms</span>
              </div>
              <div
                className={`console__reply ${
                  item.error ? "console__reply--error" : ""
                }`}
              >
                {item.output}
              </div>
            </div>
          ))
        )}
      </div>
      <div className="console__bar">
        <IconTerminal size={14} />
        <input
          className="console__input"
          value={command}
          placeholder={placeholder}
          onChange={(event) => setCommand(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void run();
            }
          }}
          disabled={busy}
          aria-label="Command"
        />
        <button
          type="button"
          className="btn btn--primary btn--sm"
          onClick={() => void run()}
          disabled={busy || !command.trim()}
        >
          {busy ? "Running…" : "Run"}
        </button>
      </div>
    </div>
  );
}

// ---- stats -------------------------------------------------------------

function StatsPanel({ session }: { session: SessionDescriptor }) {
  const toast = useToast();
  const [stats, setStats] = useState<StatItem[] | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setStats(await ipc.serverStats(session.session_id));
    } catch (error) {
      toast.error(
        "Could not read stats",
        error instanceof ipc.IpcError ? error.message : String(error),
      );
      setStats([]);
    } finally {
      setLoading(false);
    }
  }, [session.session_id, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const grouped = useMemo(() => {
    const map = new Map<string, StatItem[]>();
    for (const item of stats ?? []) {
      const list = map.get(item.section) ?? [];
      list.push(item);
      map.set(item.section, list);
    }
    return [...map.entries()];
  }, [stats]);

  return (
    <div className="stats">
      <div className="row row--between">
        <span className="pane__title">
          {loading ? "Loading…" : `${stats?.length ?? 0} values`}
        </span>
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          onClick={() => void load()}
        >
          <IconRefresh size={12} />
          Refresh
        </button>
      </div>
      {grouped.map(([section, items]) => (
        <section className="stats__section" key={section}>
          <h3 className="stats__title">{section}</h3>
          {items.map((item) => (
            <div className="stats__row" key={`${section}-${item.key}`}>
              <span className="stats__key">{item.key}</span>
              <span className="stats__value">{item.value}</span>
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}
