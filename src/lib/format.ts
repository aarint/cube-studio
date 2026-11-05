/**
 * Formatting helpers for values that cross the IPC boundary.
 *
 * Kept free of React so they can be unit tested directly.
 */

/** Human readable byte size. */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
    return "—";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

/**
 * Render a TTL in seconds the way an operator reads it.
 *
 * `-1` means "no expiry" and `-2` means "key does not exist"; both are Redis
 * conventions that also show up for other backends, so they are handled here.
 */
export function formatTtl(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) {
    return "—";
  }
  if (seconds === -1) {
    return "persistent";
  }
  if (seconds === -2) {
    return "missing";
  }
  if (seconds < 0) {
    return "unknown";
  }
  if (seconds === 0) {
    return "expiring";
  }
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (days > 0) {
    return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
  }
  if (hours > 0) {
    return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
  }
  if (minutes > 0) {
    return secs > 0 ? `${minutes}m ${secs}s` : `${minutes}m`;
  }
  return `${secs}s`;
}

/** Format a millisecond duration. */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${Math.round(ms)} ms`;
  }
  return `${(ms / 1000).toFixed(2)} s`;
}

/** Format an ISO timestamp as a compact local time. */
export function formatTimestamp(iso: string | null | undefined): string {
  if (!iso) {
    return "—";
  }
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Shorten a long value for one-line previews. */
export function truncate(text: string, max = 80): string {
  const single = text.replace(/\s+/g, " ").trim();
  if (single.length <= max) {
    return single;
  }
  return `${single.slice(0, max - 1)}…`;
}

/**
 * Decode a payload for display.
 *
 * Prefers the text rendering the backend produced; falls back to a hex preview
 * for binary data so the user sees *something* meaningful.
 */
export function payloadText(payload: {
  text?: string | null;
  base64: string;
  binary: boolean;
  json?: unknown;
}): string {
  if (payload.json !== undefined && payload.json !== null) {
    return JSON.stringify(payload.json, null, 2);
  }
  if (payload.text !== undefined && payload.text !== null) {
    return payload.text;
  }
  return hexPreview(payload.base64);
}

/** First bytes of a base64 payload rendered as hex, for binary data. */
export function hexPreview(base64: string, maxBytes = 128): string {
  if (!base64) {
    return "(empty)";
  }
  let bytes: Uint8Array;
  try {
    bytes = base64ToBytes(base64);
  } catch {
    return "(undecodable payload)";
  }
  const slice = bytes.subarray(0, maxBytes);
  const hex = Array.from(slice)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join(" ");
  return bytes.length > maxBytes ? `${hex} …` : hex;
}

/** Decode base64 into bytes without relying on Node's Buffer. */
export function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

/** Encode bytes as base64, for round-tripping binary edits. */
export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

/**
 * Pretty-print JSON when the text parses, otherwise return it untouched.
 *
 * The editor uses this so a user pasting minified JSON gets a readable draft.
 */
export function prettyJson(text: string): string {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return text;
  }
  try {
    return JSON.stringify(JSON.parse(trimmed), null, 2);
  } catch {
    return text;
  }
}

/** True when the text looks like a JSON object or array. */
export function looksLikeJson(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return false;
  }
  try {
    JSON.parse(trimmed);
    return true;
  } catch {
    return false;
  }
}

/** Display label for a backend kind. */
export const BACKEND_LABELS: Record<string, string> = {
  redis: "Redis",
  memcached: "Memcached",
  kafka: "Kafka",
  rabbitmq: "RabbitMQ",
  nats: "NATS",
  mqtt: "MQTT",
};

/** Short description shown in the connection picker. */
export const BACKEND_BLURBS: Record<string, string> = {
  redis: "Key/value store, pub/sub and keyspace events",
  memcached: "In-memory cache",
  kafka: "Distributed log with partitions and offsets",
  rabbitmq: "AMQP broker with queues and exchanges",
  nats: "Subjects, plus JetStream when enabled",
  mqtt: "Publish/subscribe for devices",
};
