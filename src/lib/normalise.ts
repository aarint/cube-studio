/**
 * Normalisers for data crossing the IPC boundary.
 *
 * Why this layer exists: the Rust side marks optional collections with
 * `skip_serializing_if = "Vec::is_empty"` / `"BTreeMap::is_empty"`, so an empty
 * `notes` or `meta` is *absent* from the JSON rather than `[]` or `{}`. Treating
 * those as always-present made the UI crash on its first real connection: a
 * Redis session has no capability notes, `caps.notes.length` threw, and React
 * unmounted the whole tree — the window went blank.
 *
 * Fixing the types alone would be a lie: the field genuinely may not arrive. So
 * every payload is passed through a normaliser that materialises the absent
 * collections. The normalisers are total (they accept `undefined`) and never
 * guess at values that are present, so they cannot mask a different bug.
 */

import type {
  Capabilities,
  ConnectionInfo,
  Container,
  Entry,
  EntryPage,
  SessionDescriptor,
} from "@/types/api";

/** Defaults used when the backend omits capabilities entirely. */
const DEFAULT_CAPABILITIES: Capabilities = {
  kind: "redis",
  containers: true,
  list_entries: true,
  write_entries: false,
  delete_entries: false,
  ttl: false,
  console: false,
  tail: false,
  publish: false,
  stats: false,
  flush: false,
  notes: [],
};

/**
 * Fill in any capability fields the backend omitted.
 *
 * Booleans default to `false` rather than `true`: if the backend did not say a
 * feature exists, the UI must not offer it.
 */
export function normaliseCapabilities(
  input: Partial<Capabilities> | null | undefined,
): Capabilities {
  if (!input) {
    return { ...DEFAULT_CAPABILITIES };
  }
  return {
    kind: input.kind ?? DEFAULT_CAPABILITIES.kind,
    containers: input.containers ?? false,
    list_entries: input.list_entries ?? false,
    write_entries: input.write_entries ?? false,
    delete_entries: input.delete_entries ?? false,
    ttl: input.ttl ?? false,
    console: input.console ?? false,
    tail: input.tail ?? false,
    publish: input.publish ?? false,
    stats: input.stats ?? false,
    flush: input.flush ?? false,
    // The crash this module exists for.
    notes: input.notes ?? [],
  };
}

export function normaliseConnectionInfo(
  input: Partial<ConnectionInfo> | null | undefined,
): ConnectionInfo {
  return {
    kind: input?.kind ?? DEFAULT_CAPABILITIES.kind,
    server_version: input?.server_version ?? null,
    capabilities: normaliseCapabilities(input?.capabilities),
    default_container: input?.default_container ?? null,
  };
}

export function normaliseContainer(input: Container): Container {
  return {
    ...input,
    meta: input.meta ?? {},
  };
}

export function normaliseEntry(input: Entry): Entry {
  return {
    ...input,
    meta: input.meta ?? {},
  };
}

export function normaliseEntryPage(
  input: Partial<EntryPage> | null | undefined,
): EntryPage {
  return {
    entries: (input?.entries ?? []).map(normaliseEntry),
    next_cursor: input?.next_cursor ?? null,
    total: input?.total ?? null,
  };
}

export function normaliseSession(input: SessionDescriptor): SessionDescriptor {
  return {
    ...input,
    endpoints: input.endpoints ?? [],
    info: normaliseConnectionInfo(input.info),
  };
}
