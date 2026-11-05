/**
 * Thin typed wrapper over the Tauri IPC boundary.
 *
 * Every command in `src-tauri/src/commands.rs` has exactly one function here.
 * Errors are normalised into an [`AppError`] so callers never have to parse
 * strings to find out what kind of failure happened.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppError,
  AppMetadata,
  ConnectionConfig,
  ConnectionInfo,
  ConsoleResult,
  Container,
  Entry,
  EntryPage,
  ErrorKind,
  MutationResult,
  SessionDescriptor,
  StatItem,
  StreamMessage,
  TailDescriptor,
  WriteOptions,
} from "@/types/api";

/** Event names emitted by the Rust side. */
const STREAM_EVENT = "cube://stream-message";
const STREAM_CLOSED_EVENT = "cube://stream-closed";

/** Thrown when a command fails. Carries the backend's structured error. */
export class IpcError extends Error {
  readonly kind: ErrorKind;
  readonly detail?: string;

  constructor(error: AppError) {
    super(error.message);
    this.name = "IpcError";
    this.kind = error.kind;
    if (error.detail) {
      this.detail = error.detail;
    }
  }

  /** True when retrying the same call could plausibly succeed. */
  get retryable(): boolean {
    return this.kind === "network" || this.kind === "timeout";
  }

  /** True when the user has to change something before retrying. */
  get needsUserAction(): boolean {
    return this.kind === "invalid_input" || this.kind === "unauthorized";
  }
}

/**
 * Tauri rejects with the serialized `AppError` string. Parse it back, and fall
 * back to a generic error if the payload is something else (a panic, for
 * instance).
 */
function toIpcError(raw: unknown): IpcError {
  if (raw instanceof IpcError) {
    return raw;
  }
  if (typeof raw === "string") {
    try {
      const parsed = JSON.parse(raw) as AppError;
      if (parsed && typeof parsed.message === "string" && parsed.kind) {
        return new IpcError(parsed);
      }
    } catch {
      // Not JSON: fall through and wrap the raw text.
    }
    return new IpcError({ kind: "internal", message: raw });
  }
  if (raw instanceof Error) {
    return new IpcError({ kind: "internal", message: raw.message });
  }
  return new IpcError({
    kind: "internal",
    message: "the backend returned an unrecognised error",
  });
}

/**
 * Resolve the active transport.
 *
 * In a Tauri window this is always `invoke`. When the frontend is opened in a
 * plain browser during development, `preview.ts` installs an in-memory
 * replacement so the UI is still fully explorable.
 */
type PreviewModule = typeof import("@/lib/preview");

function previewBackend(): PreviewModule["getPreviewBackend"] | null {
  if (!import.meta.env.DEV) {
    return null;
  }
  const found = (window as unknown as Record<string, unknown>).__CUBE_PREVIEW__;
  return found ? () => found as ReturnType<PreviewModule["getPreviewBackend"]> : null;
}

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const preview = previewBackend();
  if (preview) {
    const backend = preview();
    if (backend) {
      const previewName = PREVIEW_METHODS[command];
      const fn = previewName
        ? (backend as unknown as Record<string, unknown>)[previewName]
        : undefined;
      if (typeof fn !== "function") {
        throw toIpcError(
          JSON.stringify({
            kind: "unsupported",
            message: `the preview backend does not implement ${command}`,
          }),
        );
      }
      try {
        return (await (fn as (...params: unknown[]) => unknown)(...argsToParams(command, args))) as T;
      } catch (raw) {
        throw toIpcError(raw);
      }
    }
  }
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw toIpcError(raw);
  }
}

/**
 * Tauri command names are snake_case; the preview backend exposes camelCase
 * methods. This is the single translation table.
 */
const PREVIEW_METHODS: Record<string, string> = {
  app_metadata: "appMetadata",
  list_profiles: "listProfiles",
  save_profile: "saveProfile",
  delete_profile: "deleteProfile",
  connect: "connect",
  disconnect: "disconnect",
  list_sessions: "listSessions",
  session_info: "sessionInfo",
  test_connection: "testConnection",
  list_containers: "listContainers",
  list_entries: "listEntries",
  read_entry: "readEntry",
  write_entry: "writeEntry",
  delete_entries: "deleteEntries",
  set_entry_ttl: "setEntryTtl",
  publish_message: "publishMessage",
  flush_container: "flushContainer",
  server_stats: "serverStats",
  run_console: "runConsole",
  start_tail: "startTail",
  stop_tail: "stopTail",
  list_tails: "listTails",
};

/**
 * The preview backend exposes plain functions with positional arguments, while
 * `invoke` takes a named argument object. Bridge the two here so the preview
 * code stays readable.
 */
function argsToParams(
  command: string,
  args: Record<string, unknown> = {},
): unknown[] {
  switch (command) {
    case "list_profiles":
    case "list_sessions":
    case "list_tails":
    case "app_metadata":
      return [];
    case "save_profile":
      return [args.config];
    case "delete_profile":
      return [args.id];
    case "connect":
    case "test_connection":
      return [args.config];
    case "disconnect":
    case "session_info":
    case "list_containers":
    case "server_stats":
      return [args.sessionId];
    case "list_entries":
      return [
        args.sessionId,
        args.container,
        {
          cursor: args.cursor ?? null,
          limit: args.limit ?? 200,
          search: args.search ?? null,
        },
      ];
    case "read_entry":
      return [args.sessionId, args.container, args.entry];
    case "write_entry":
      return [
        args.sessionId,
        args.container,
        args.entry,
        { text: args.text ?? undefined, base64: args.payloadBase64 ?? undefined },
        args.options ?? undefined,
      ];
    case "delete_entries":
      return [args.sessionId, args.container, args.entries];
    case "set_entry_ttl":
      return [args.sessionId, args.container, args.entry, args.ttlSecs];
    case "publish_message":
      return [
        args.sessionId,
        args.container,
        { text: args.text ?? undefined, base64: args.payloadBase64 ?? undefined },
        args.options ?? undefined,
      ];
    case "flush_container":
      return [args.sessionId, args.container];
    case "run_console":
      return [args.sessionId, args.command];
    case "start_tail":
      return [args.sessionId, args.container];
    case "stop_tail":
      return [args.subscriptionId];
    default:
      return [args];
  }
}

// ---- metadata & profiles ----------------------------------------------

export const appMetadata = () => call<AppMetadata>("app_metadata");

export const listProfiles = () => call<ConnectionConfig[]>("list_profiles");

export const saveProfile = (config: ConnectionConfig) =>
  call<ConnectionConfig>("save_profile", { config });

export const deleteProfile = (id: string) =>
  call<void>("delete_profile", { id });

// ---- connections ------------------------------------------------------

export const connect = (config: ConnectionConfig) =>
  call<SessionDescriptor>("connect", { config });

export const disconnect = (sessionId: string) =>
  call<void>("disconnect", { sessionId });

export const listSessions = () => call<SessionDescriptor[]>("list_sessions");

export const sessionInfo = (sessionId: string) =>
  call<ConnectionInfo>("session_info", { sessionId });

export const testConnection = (config: ConnectionConfig) =>
  call<ConnectionInfo>("test_connection", { config });

// ---- browsing ---------------------------------------------------------

export const listContainers = (sessionId: string) =>
  call<Container[]>("list_containers", { sessionId });

export const listEntries = (
  sessionId: string,
  container: string,
  options: { cursor?: string | null; limit?: number; search?: string | null } = {},
) =>
  call<EntryPage>("list_entries", {
    sessionId,
    container,
    cursor: options.cursor ?? null,
    limit: options.limit ?? 200,
    search: options.search ?? null,
  });

export const readEntry = (sessionId: string, container: string, entry: string) =>
  call<Entry>("read_entry", { sessionId, container, entry });

/** Payload accepted by the write/publish commands. */
export interface PayloadInput {
  /** Plain text; ignored when `base64` is set. */
  text?: string;
  /** Base64 of raw bytes, for binary payloads. */
  base64?: string;
}

function payloadArgs(payload: PayloadInput) {
  return {
    text: payload.text ?? null,
    payloadBase64: payload.base64 ?? null,
  };
}

export const writeEntry = (
  sessionId: string,
  container: string,
  entry: string,
  payload: PayloadInput,
  options?: WriteOptions,
) =>
  call<MutationResult>("write_entry", {
    sessionId,
    container,
    entry,
    ...payloadArgs(payload),
    options: options ?? null,
  });

export const deleteEntries = (
  sessionId: string,
  container: string,
  entries: string[],
) => call<MutationResult>("delete_entries", { sessionId, container, entries });

export const setEntryTtl = (
  sessionId: string,
  container: string,
  entry: string,
  ttlSecs: number,
) => call<void>("set_entry_ttl", { sessionId, container, entry, ttlSecs });

export const publishMessage = (
  sessionId: string,
  container: string,
  payload: PayloadInput,
  options?: WriteOptions,
) =>
  call<MutationResult>("publish_message", {
    sessionId,
    container,
    ...payloadArgs(payload),
    options: options ?? null,
  });

export const flushContainer = (sessionId: string, container: string) =>
  call<MutationResult>("flush_container", { sessionId, container });

export const serverStats = (sessionId: string) =>
  call<StatItem[]>("server_stats", { sessionId });

export const runConsole = (sessionId: string, command: string) =>
  call<ConsoleResult>("run_console", { sessionId, command });

// ---- live streaming ---------------------------------------------------

export const startTail = (sessionId: string, container: string) =>
  call<string>("start_tail", { sessionId, container });

export const stopTail = (subscriptionId: string) =>
  call<void>("stop_tail", { subscriptionId });

export const listTails = () => call<TailDescriptor[]>("list_tails");

/** Subscribe to streamed messages. Returns a function that unlistens. */
export function onStreamMessage(
  handler: (message: StreamMessage) => void,
): Promise<UnlistenFn> {
  const preview = previewBackend()?.() as
    | { onStreamMessage: (h: (m: StreamMessage) => void) => Promise<() => void> }
    | null
    | undefined;
  if (preview) {
    return preview.onStreamMessage(handler);
  }
  return listen<StreamMessage>(STREAM_EVENT, (event) => handler(event.payload));
}

export interface StreamClosedEvent {
  subscription_id: string;
  messages: number;
}

export function onStreamClosed(
  handler: (event: StreamClosedEvent) => void,
): Promise<UnlistenFn> {
  const preview = previewBackend()?.() as
    | {
        onStreamClosed: (h: (e: StreamClosedEvent) => void) => Promise<() => void>;
      }
    | null
    | undefined;
  if (preview) {
    return preview.onStreamClosed(handler);
  }
  return listen<StreamClosedEvent>(STREAM_CLOSED_EVENT, (event) =>
    handler(event.payload),
  );
}
