/**
 * Development-only in-memory backend.
 *
 * Opening the frontend in a plain browser is the fastest way to iterate on
 * layout, but the real data layer lives behind Tauri IPC. This module implements
 * the same surface as `@/lib/ipc` with realistic fixtures for every backend, so
 * `npm run dev:web` gives a fully clickable app.
 *
 * It is excluded from production builds: `main.tsx` only imports it under
 * `import.meta.env.DEV`, and `ipc.ts` only consults the installed override when
 * `window.__CUBE_PREVIEW__` is present.
 */

import type {
  AppMetadata,
  ConnectionConfig,
  ConnectionInfo,
  ConsoleResult,
  Container,
  Entry,
  EntryPage,
  MutationResult,
  SessionDescriptor,
  StatItem,
  StreamMessage,
  Payload,
  TailDescriptor,
  WriteOptions,
} from "@/types/api";
import type { PayloadInput, StreamClosedEvent } from "@/lib/ipc";

const SENTINEL_KEY = "__CUBE_PREVIEW__";

function caps(
  kind: ConnectionInfo["kind"],
  overrides: Partial<ConnectionInfo["capabilities"]> = {},
): ConnectionInfo["capabilities"] {
  return {
    kind,
    containers: true,
    list_entries: true,
    write_entries: true,
    delete_entries: true,
    ttl: false,
    console: false,
    tail: true,
    publish: false,
    stats: true,
    flush: false,
    notes: [],
    ...overrides,
  };
}

const METADATA: AppMetadata = {
  version: "2.0.0-preview",
  platform: "preview",
  backends: [
    {
      kind: "redis",
      label: "Redis",
      default_port: 6379,
      capabilities: caps("redis", { ttl: true, console: true, flush: true }),
      fields: ["username", "password", "redis_mode", "redis_db"],
    },
    {
      kind: "memcached",
      label: "Memcached",
      default_port: 11211,
      capabilities: caps("memcached", {
        containers: false,
        console: true,
        ttl: true,
        flush: true,
        notes: [
          "Memcached has no key enumeration command. Keys come from stats, so only resident keys appear.",
        ],
      }),
      fields: [],
    },
    {
      kind: "kafka",
      label: "Kafka",
      default_port: 9092,
      capabilities: caps("kafka", {
        write_entries: false,
        delete_entries: false,
        publish: true,
        notes: ["Kafka topics are append-only: messages cannot be edited."],
      }),
      fields: ["username", "password", "kafka_sasl_mechanism"],
    },
    {
      kind: "rabbitmq",
      label: "RabbitMQ",
      default_port: 5672,
      capabilities: caps("rabbitmq", {
        write_entries: false,
        delete_entries: false,
        publish: true,
        console: true,
        notes: ["Publishing goes through an exchange with a routing key."],
      }),
      fields: ["username", "password", "vhost"],
    },
    {
      kind: "nats",
      label: "NATS",
      default_port: 4222,
      capabilities: caps("nats", {
        write_entries: false,
        delete_entries: false,
        publish: true,
        console: true,
        notes: ["Core NATS keeps no history; use the live view."],
      }),
      fields: ["username", "password", "token"],
    },
    {
      kind: "mqtt",
      label: "MQTT",
      default_port: 1883,
      capabilities: caps("mqtt", { ttl: true, console: true }),
      fields: ["username", "password", "client_id"],
    },
  ],
};

function makePayload(text: string): Payload {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  let json: unknown = undefined;
  const trimmed = text.trim();
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      json = JSON.parse(trimmed);
    } catch {
      json = undefined;
    }
  }
  return {
    text,
    base64: btoa(binary),
    size: bytes.length,
    truncated: false,
    binary: false,
    json,
  };
}

function entry(
  id: string,
  kind: string,
  value: string,
  ttl: number | null,
  size?: number,
  meta: Record<string, string> = {},
): Entry {
  return {
    id,
    label: id,
    kind,
    size: size ?? new TextEncoder().encode(value).length,
    ttl,
    timestamp: null,
    meta,
    payload: makePayload(value),
  };
}

const REDIS_DB0: Entry[] = [
  entry(
    "user:1001",
    "hash",
    '{\n  "id": "1001",\n  "name": "Ada Lovelace",\n  "plan": "pro",\n  "created": "2024-02-11T09:24:00Z"\n}',
    -1,
    186,
  ),
  entry(
    "session:9f2c7a",
    "string",
    "eyJ1aWQiOjEwMDEsImV4cCI6MTczOTk5OTk5OX0=",
    1820,
    96,
  ),
  entry(
    "orders:2024:q1",
    "list",
    '[\n  "order-8812",\n  "order-8813",\n  "order-8817"\n]',
    -1,
    148,
  ),
  entry("rate:limit:1001", "string", "42", 58, 4),
  entry(
    "feature:flags",
    "hash",
    '{\n  "new_checkout": "on",\n  "beta_search": "off",\n  "dark_mode": "on"\n}',
    -1,
    132,
  ),
  entry(
    "tags:article:88",
    "set",
    '[\n  "rust",\n  "tauri",\n  "middleware"\n]',
    -1,
    92,
  ),
  entry(
    "leaderboard",
    "zset",
    '[\n  { "member": "ada", "score": 9810 },\n  { "member": "grace", "score": 9420 }\n]',
    -1,
    210,
  ),
  entry("metrics:ingest", "stream", '[\n  "1712-0",\n  "1713-0"\n]', -1, 164),
  entry(
    "cache:product:77",
    "string",
    '{"sku":"SKU-77","price":49.9,"stock":12}',
    340,
    78,
  ),
  entry("lock:job:nightly", "string", "worker-3", 12, 8),
];

const KAFKA_ENTRIES: Entry[] = Array.from({ length: 12 }, (_, index) => {
  const offset = 2418 - index;
  const at = new Date(Date.now() - index * 4200).toISOString();
  const body = JSON.stringify(
    {
      type: "order.created",
      id: offset,
      customer: "cust-4412",
      total: 129.5,
      currency: "EUR",
      at,
    },
    null,
    2,
  );
  const item = entry(`0:${offset}`, "message", body, null, undefined, {
    partition: "0",
    offset: String(offset),
    key: `evt-${offset}`,
    preview: `{"type":"order.created","id":${offset}}`,
  });
  item.label = `offset ${offset}`;
  item.timestamp = at;
  return item;
});

const MQTT_ENTRIES: Entry[] = [
  ["sensors/floor1/temp", "21.5"],
  ["sensors/floor1/humidity", "48"],
  ["sensors/floor2/temp", "19.8"],
  ["devices/pump-3/status", "online"],
].map(([topic, value]) =>
  entry(topic!, "message", value!, null, undefined, {
    retained: "true",
    preview: value!,
  }),
);

interface MockSession {
  descriptor: SessionDescriptor;
  containers: Container[];
  entries: Map<string, Entry[]>;
  stats: StatItem[];
  consoleReply: (command: string) => ConsoleResult;
}

function buildSession(
  config: ConnectionConfig,
  sessionId: string,
): MockSession {
  const kind = config.kind;
  const backend = METADATA.backends.find((item) => item.kind === kind);
  const capabilities = backend?.capabilities ?? caps("redis");

  const descriptor: SessionDescriptor = {
    session_id: sessionId,
    profile_id: config.id || null,
    name: config.name || `${config.host}:${config.port}`,
    kind,
    endpoints: [`${config.host}:${config.port}`],
    info: {
      kind,
      server_version:
        kind === "redis"
          ? "7.2.4"
          : kind === "kafka"
            ? "3.7.0"
            : kind === "mqtt"
              ? "mosquitto 2.0.18"
              : kind === "rabbitmq"
                ? "3.13.7"
                : kind === "nats"
                  ? "JetStream"
                  : "1.6.21",
      capabilities,
      default_container:
        kind === "redis"
          ? "db:0"
          : kind === "kafka"
            ? "orders"
            : kind === "mqtt"
              ? "topic:#"
              : kind === "memcached"
                ? "server"
                : null,
    },
  };

  const containers: Container[] = [];
  const entries = new Map<string, Entry[]>();

  if (kind === "redis") {
    for (let index = 0; index < 4; index += 1) {
      const id = `db:${index}`;
      containers.push({
        id,
        name: `DB${index}`,
        kind: "redis_db",
        meta: {},
        size: index === 0 ? REDIS_DB0.length : 0,
      });
      entries.set(id, index === 0 ? [...REDIS_DB0] : []);
    }
  } else if (kind === "kafka") {
    for (const [name, partitions] of [
      ["orders", 3],
      ["payments", 1],
      ["audit-log", 2],
      ["dead-letter", 1],
    ] as const) {
      containers.push({
        id: name,
        name,
        kind: "kafka_topic",
        meta: { partitions: String(partitions) },
        size: null,
      });
      entries.set(name, name === "orders" ? [...KAFKA_ENTRIES] : []);
    }
  } else if (kind === "mqtt") {
    containers.push({
      id: "topic:#",
      name: "# (all topics)",
      kind: "mqtt_topic",
      meta: { filter: "#" },
      size: null,
    });
    entries.set("topic:#", [...MQTT_ENTRIES]);
  } else if (kind === "rabbitmq") {
    for (const name of ["order-events", "email-outbox", "dead-letter"]) {
      containers.push({
        id: `queue:${name}`,
        name,
        kind: "rabbit_queue",
        meta: { messages: "0", consumers: "1" },
        size: 0,
      });
      entries.set(`queue:${name}`, []);
    }
    entries.set("queue:order-events", [
      entry("0", "message", '{"order":8812}', null, 17, {
        routing_key: "orders.created",
        preview: '{"order":8812}',
      }),
      entry("1", "message", '{"order":8813}', null, 17, {
        routing_key: "orders.created",
        preview: '{"order":8813}',
      }),
    ]);
  } else if (kind === "nats") {
    containers.push({
      id: "subject:>",
      name: "> (live subject)",
      kind: "nats_subject",
      meta: {},
      size: null,
    });
    entries.set("subject:>", []);
  } else {
    containers.push({
      id: "server",
      name: descriptor.name,
      kind: "memcached_server",
      meta: { curr_items: "3" },
      size: 3,
    });
    entries.set("server", [
      entry("session:abc", "string", "cached-value", 240, 12),
      entry("counter:hits", "string", "10921", -1, 5),
    ]);
  }

  const stats: StatItem[] = [
    {
      section: "server",
      key: "version",
      value: descriptor.info.server_version ?? "",
    },
    { section: "server", key: "uptime_in_seconds", value: "482913" },
    { section: "memory", key: "used_memory", value: "12.4M" },
    { section: "memory", key: "used_memory_peak", value: "18.1M" },
    { section: "clients", key: "connected_clients", value: "24" },
    {
      section: "stats",
      key: "total_connections_received",
      value: "1834291",
    },
    { section: "stats", key: "keyspace_hits", value: "98432112" },
    { section: "stats", key: "keyspace_misses", value: "41209" },
  ];

  return {
    descriptor,
    containers,
    entries,
    stats,
    consoleReply: (command) => ({
      output: command.trim().toUpperCase().startsWith("PING")
        ? "PONG"
        : `(preview) ${command}`,
      error: false,
      elapsed_ms: 1,
    }),
  };
}

// ---- mutable state ----------------------------------------------------

let nextSession = 1;
const sessions = new Map<string, MockSession>();
const profiles = new Map<string, ConnectionConfig>();
const tails = new Map<string, number>();
const streamListeners = new Set<(message: StreamMessage) => void>();
const closedListeners = new Set<(event: StreamClosedEvent) => void>();

function profileKey(config: ConnectionConfig): string {
  return config.id || `preview-${config.kind}-${config.host}-${config.port}`;
}

function previewError(kind: string, message: string): never {
  throw JSON.stringify({ kind, message });
}

function requireSession(sessionId: string): MockSession {
  const session = sessions.get(sessionId);
  if (!session) {
    previewError("no_session", "this preview session has been closed");
  }
  return session;
}

function fakeSource(kind: string, target: string): string {
  switch (kind) {
    case "redis":
      return target.replace(/^channel:/, "").replace(/\*$/, "orders.created");
    case "mqtt":
      return target.replace(/^topic:/, "").replace(/#$/, "sensors/floor1/temp");
    case "nats":
      return target.replace(/^subject:/, "");
    default:
      return target;
  }
}

function startFakeStream(subscriptionId: string, session: MockSession, target: string) {
  const timer = window.setInterval(() => {
    const message: StreamMessage = {
      subscription_id: subscriptionId,
      source: fakeSource(session.descriptor.kind, target),
      payload: makePayload(
        JSON.stringify({
          event: "tick",
          at: new Date().toISOString(),
          value: Math.round(Math.random() * 1000) / 10,
        }),
      ),
      key: null,
      headers: {},
      timestamp: new Date().toISOString(),
      position: String(Math.floor(Math.random() * 100000)),
    };
    for (const listener of streamListeners) listener(message);
  }, 1500);
  tails.set(subscriptionId, timer);
}

export interface PreviewBackend {
  appMetadata: typeof appMetadata;
  listProfiles: typeof listProfiles;
  saveProfile: typeof saveProfile;
  deleteProfile: typeof deleteProfile;
  connect: typeof connect;
  disconnect: typeof disconnect;
  listSessions: typeof listSessions;
  sessionInfo: typeof sessionInfo;
  testConnection: typeof testConnection;
  listContainers: typeof listContainers;
  listEntries: typeof listEntries;
  readEntry: typeof readEntry;
  writeEntry: typeof writeEntry;
  deleteEntries: typeof deleteEntries;
  setEntryTtl: typeof setEntryTtl;
  publishMessage: typeof publishMessage;
  flushContainer: typeof flushContainer;
  serverStats: typeof serverStats;
  runConsole: typeof runConsole;
  startTail: typeof startTail;
  stopTail: typeof stopTail;
  listTails: typeof listTails;
  onStreamMessage: typeof onStreamMessage;
  onStreamClosed: typeof onStreamClosed;
}

// Declared after use in the interface above; the implementations follow.
async function appMetadata(): Promise<AppMetadata> {
  return METADATA;
}

async function listProfiles(): Promise<ConnectionConfig[]> {
  return [...profiles.values()];
}

async function saveProfile(config: ConnectionConfig): Promise<ConnectionConfig> {
  const id = config.id || profileKey(config);
  const saved = { ...config, id };
  profiles.set(id, saved);
  return saved;
}

async function deleteProfile(id: string): Promise<void> {
  profiles.delete(id);
}

async function testConnection(
  config: ConnectionConfig,
): Promise<ConnectionInfo> {
  if (config.host.trim() === "fail") {
    previewError("network", "connection refused (preview: host 'fail' always fails)");
  }
  return buildSession(config, "probe").descriptor.info;
}

async function connect(config: ConnectionConfig): Promise<SessionDescriptor> {
  if (config.host.trim() === "fail") {
    previewError("network", "connection refused (preview: host 'fail' always fails)");
  }
  const id = `preview-${nextSession}`;
  nextSession += 1;
  const session = buildSession(config, id);
  sessions.set(id, session);
  if (config.meta?.save) {
    await saveProfile(config);
  }
  return session.descriptor;
}

async function disconnect(sessionId: string): Promise<void> {
  sessions.delete(sessionId);
  for (const [subscriptionId, timer] of tails) {
    if (subscriptionId.startsWith(sessionId)) {
      window.clearInterval(timer);
      tails.delete(subscriptionId);
    }
  }
}

async function listSessions(): Promise<SessionDescriptor[]> {
  return [...sessions.values()].map((session) => session.descriptor);
}

async function sessionInfo(sessionId: string): Promise<ConnectionInfo> {
  return requireSession(sessionId).descriptor.info;
}

async function listContainers(sessionId: string): Promise<Container[]> {
  return requireSession(sessionId).containers;
}

async function listEntries(
  sessionId: string,
  container: string,
  options: { cursor?: string | null; limit?: number; search?: string | null } = {},
): Promise<EntryPage> {
  const session = requireSession(sessionId);
  const all = session.entries.get(container) ?? [];
  const needle = (options.search ?? "").trim().toLowerCase();
  const filtered = needle
    ? all.filter((item) => item.label.toLowerCase().includes(needle))
    : all;
  const limit = options.limit ?? 200;
  const start = options.cursor ? Number(options.cursor) : 0;
  const slice = filtered.slice(start, start + limit);
  const nextIndex = start + slice.length;
  return {
    entries: slice,
    next_cursor: nextIndex < filtered.length ? String(nextIndex) : null,
    total: filtered.length,
  };
}

async function readEntry(
  sessionId: string,
  container: string,
  id: string,
): Promise<Entry> {
  const session = requireSession(sessionId);
  const found = (session.entries.get(container) ?? []).find(
    (item) => item.id === id,
  );
  if (!found) {
    previewError("server_error", "entry no longer exists");
  }
  return found as Entry;
}

async function writeEntry(
  sessionId: string,
  container: string,
  id: string,
  payload: PayloadInput,
  options?: WriteOptions,
): Promise<MutationResult> {
  const session = requireSession(sessionId);
  const list = session.entries.get(container) ?? [];
  const body = payload.text ?? "";
  const existing = list.find((item) => item.id === id);

  if (existing) {
    existing.payload = makePayload(body);
    existing.size = new TextEncoder().encode(body).length;
    if (options?.ttl !== null && options?.ttl !== undefined) {
      existing.ttl = options.ttl;
    }
  } else if (options?.entry_kind === "hash" || options?.entry_kind === "list") {
    // Mirrors the backend: structured types require JSON.
    const trimmed = body.trim();
    if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
      previewError(
        "invalid_input",
        `${options.entry_kind} values must be written as JSON`,
      );
    }
    list.unshift(
      entry(id, options.entry_kind, body, options.ttl ?? -1),
    );
    session.entries.set(container, list);
  } else {
    list.unshift(entry(id, options?.entry_kind ?? "string", body, options?.ttl ?? -1));
    session.entries.set(container, list);
  }

  return { ok: true, message: `saved ${id}`, affected: [id] };
}

async function deleteEntries(
  sessionId: string,
  container: string,
  ids: string[],
): Promise<MutationResult> {
  const session = requireSession(sessionId);
  const list = session.entries.get(container) ?? [];
  session.entries.set(
    container,
    list.filter((item) => !ids.includes(item.id)),
  );
  return { ok: true, message: `deleted ${ids.length}`, affected: ids };
}

async function setEntryTtl(
  sessionId: string,
  container: string,
  id: string,
  ttlSecs: number,
): Promise<void> {
  const session = requireSession(sessionId);
  const found = (session.entries.get(container) ?? []).find(
    (item) => item.id === id,
  );
  if (found) found.ttl = ttlSecs;
}

async function publishMessage(
  sessionId: string,
  container: string,
  payload: PayloadInput,
  options?: WriteOptions,
): Promise<MutationResult> {
  requireSession(sessionId);
  const size = new TextEncoder().encode(payload.text ?? "").length;
  return {
    ok: true,
    message: `published ${size} byte(s) to ${
      options?.routing_key ?? container
    }`,
    affected: [],
  };
}

async function flushContainer(
  sessionId: string,
  container: string,
): Promise<MutationResult> {
  const session = requireSession(sessionId);
  session.entries.set(container, []);
  return { ok: true, message: "container flushed", affected: [] };
}

async function serverStats(sessionId: string): Promise<StatItem[]> {
  return requireSession(sessionId).stats;
}

async function runConsole(
  sessionId: string,
  command: string,
): Promise<ConsoleResult> {
  return requireSession(sessionId).consoleReply(command);
}

async function startTail(
  sessionId: string,
  container: string,
): Promise<string> {
  const session = requireSession(sessionId);
  const id = `${sessionId}-tail-${Math.random().toString(36).slice(2, 8)}`;
  startFakeStream(id, session, container);
  return id;
}

async function stopTail(subscriptionId: string): Promise<void> {
  const timer = tails.get(subscriptionId);
  if (timer !== undefined) {
    window.clearInterval(timer);
    tails.delete(subscriptionId);
  }
}

async function listTails(): Promise<TailDescriptor[]> {
  return [...tails.keys()].map((id) => ({
    subscription_id: id,
    session_id: id.split("-tail-")[0] ?? id,
    container: "",
  }));
}

async function onStreamMessage(
  handler: (message: StreamMessage) => void,
): Promise<() => void> {
  streamListeners.add(handler);
  return () => streamListeners.delete(handler);
}

async function onStreamClosed(
  handler: (event: StreamClosedEvent) => void,
): Promise<() => void> {
  closedListeners.add(handler);
  return () => closedListeners.delete(handler);
}

const backend: PreviewBackend = {
  appMetadata,
  listProfiles,
  saveProfile,
  deleteProfile,
  connect,
  disconnect,
  listSessions,
  sessionInfo,
  testConnection,
  listContainers,
  listEntries,
  readEntry,
  writeEntry,
  deleteEntries,
  setEntryTtl,
  publishMessage,
  flushContainer,
  serverStats,
  runConsole,
  startTail,
  stopTail,
  listTails,
  onStreamMessage,
  onStreamClosed,
};

/** Install the preview backend and seed a few saved connections. */
export function installPreviewBackend() {
  (window as unknown as Record<string, unknown>)[SENTINEL_KEY] = backend;
  void Promise.all([
    saveProfile({
      ...(emptyPreviewConfig("redis")),
      id: "preview-redis",
      name: "local redis",
      host: "127.0.0.1",
      port: 6379,
    }),
    saveProfile({
      ...emptyPreviewConfig("kafka"),
      id: "preview-kafka",
      name: "events cluster",
      host: "broker.internal",
      port: 9092,
    }),
    saveProfile({
      ...emptyPreviewConfig("mqtt"),
      id: "preview-mqtt",
      name: "sensor gateway",
      host: "10.0.4.21",
      port: 1883,
    }),
  ]);
}

/** True when the preview backend is active. */
export function getPreviewBackend(): PreviewBackend | null {
  const candidate = (window as unknown as Record<string, unknown>)[SENTINEL_KEY];
  return candidate ? (candidate as PreviewBackend) : null;
}

function emptyPreviewConfig(kind: ConnectionConfig["kind"]): ConnectionConfig {
  return {
    id: "",
    name: "",
    kind,
    host: "127.0.0.1",
    port: METADATA.backends.find((item) => item.kind === kind)?.default_port ?? 0,
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
