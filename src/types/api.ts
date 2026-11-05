/**
 * Types mirroring the Rust IPC contract.
 *
 * These are hand-written rather than generated: the backend surface is small
 * and stable, and hand-writing them keeps the field names and doc comments
 * where a frontend developer looks for them. Every type here has a counterpart
 * in `src-tauri/src/model.rs` or `config.rs`.
 */

export type BackendKind =
  | "redis"
  | "memcached"
  | "kafka"
  | "rabbitmq"
  | "nats"
  | "mqtt";

export type ErrorKind =
  | "invalid_input"
  | "network"
  | "unauthorized"
  | "server_error"
  | "unsupported"
  | "no_session"
  | "timeout"
  | "internal";

/** The single error shape every command can return. */
export interface AppError {
  kind: ErrorKind;
  message: string;
  detail?: string;
}

export type TlsMode = "disabled" | "enabled" | "insecure";
export type RedisMode = "standalone" | "cluster" | "sentinel";

export interface ConnectionConfig {
  id: string;
  name: string;
  kind: BackendKind;
  host: string;
  port: number;
  extra_nodes: string[];
  username: string;
  password: string;
  tls: TlsMode;
  tls_ca_file: string | null;
  tls_skip_verify: boolean;
  redis_mode: RedisMode;
  redis_db: number;
  redis_sentinel_master: string | null;
  kafka_sasl_mechanism: string;
  kafka_group_id: string | null;
  vhost: string;
  client_id: string | null;
  keep_alive: number | null;
  token: string | null;
  timeout_secs: number | null;
  max_payload_bytes: number | null;
  meta: Record<string, string>;
}

export interface Capabilities {
  kind: BackendKind;
  containers: boolean;
  list_entries: boolean;
  write_entries: boolean;
  delete_entries: boolean;
  ttl: boolean;
  console: boolean;
  tail: boolean;
  publish: boolean;
  stats: boolean;
  flush: boolean;
  notes: string[];
}

export type ContainerKind =
  | "redis_db"
  | "memcached_server"
  | "kafka_topic"
  | "rabbit_queue"
  | "rabbit_exchange"
  | "nats_stream"
  | "nats_subject"
  | "mqtt_topic";

export interface Container {
  id: string;
  name: string;
  kind: ContainerKind;
  meta: Record<string, string>;
  size?: number | null;
}

export interface Payload {
  text?: string | null;
  base64: string;
  size: number;
  truncated: boolean;
  binary: boolean;
  json?: unknown;
}

export interface Entry {
  id: string;
  label: string;
  kind: string;
  size?: number | null;
  ttl?: number | null;
  timestamp?: string | null;
  meta: Record<string, string>;
  payload?: Payload | null;
}

export interface EntryPage {
  entries: Entry[];
  next_cursor?: string | null;
  total?: number | null;
}

export interface WriteOptions {
  entry_kind?: string | null;
  ttl?: number | null;
  routing_key?: string | null;
  exchange?: string | null;
  headers?: Record<string, string> | null;
  qos?: number | null;
  retained?: boolean | null;
  partition?: number | null;
}

export interface MutationResult {
  ok: boolean;
  message?: string | null;
  affected: string[];
}

export interface ConnectionInfo {
  kind: BackendKind;
  server_version?: string | null;
  capabilities: Capabilities;
  default_container?: string | null;
}

export interface SessionDescriptor {
  session_id: string;
  profile_id?: string | null;
  name: string;
  kind: BackendKind;
  endpoints: string[];
  info: ConnectionInfo;
}

export interface StreamMessage {
  subscription_id: string;
  source: string;
  payload: Payload;
  key?: string | null;
  headers: Record<string, string>;
  timestamp: string;
  position?: string | null;
}

export interface ConsoleResult {
  output: string;
  error: boolean;
  elapsed_ms: number;
}

export interface StatItem {
  section: string;
  key: string;
  value: string;
}

export interface BackendMetadata {
  kind: BackendKind;
  label: string;
  default_port: number;
  capabilities: Capabilities;
  fields: string[];
}

export interface AppMetadata {
  backends: BackendMetadata[];
  version: string;
  platform: string;
}

export interface TailDescriptor {
  subscription_id: string;
  session_id: string;
  container: string;
}
