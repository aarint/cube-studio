import { describe, expect, it } from "vitest";

import {
  normaliseCapabilities,
  normaliseContainer,
  normaliseEntry,
  normaliseEntryPage,
  normaliseSession,
} from "@/lib/normalise";
import type { Capabilities, Container, Entry, SessionDescriptor } from "@/types/api";

/**
 * These tests encode a real crash.
 *
 * The Rust side marks `notes` and `meta` with `skip_serializing_if`, so an empty
 * collection is omitted from the JSON. The frontend treated both as always
 * present, so opening any Redis connection — which has no capability notes —
 * threw on `caps.notes.length` and unmounted the entire React tree, leaving a
 * blank window.
 */

function capsWithoutNotes(): Capabilities {
  // What the backend actually sends for Redis: `notes` is absent, not `[]`.
  const { notes: _omitted, ...rest } = {
    kind: "redis",
    containers: true,
    list_entries: true,
    write_entries: true,
    delete_entries: true,
    ttl: true,
    console: true,
    tail: true,
    publish: false,
    stats: true,
    flush: true,
    notes: [] as string[],
  } satisfies Capabilities;
  return rest as Capabilities;
}

describe("normaliseCapabilities", () => {
  it("supplies an empty notes array when the field is omitted", () => {
    const caps = normaliseCapabilities(capsWithoutNotes());
    expect(caps.notes).toEqual([]);
    // The exact expression that used to throw.
    expect(() => caps.notes.length).not.toThrow();
  });

  it("keeps notes that are present", () => {
    const withNotes = {
      ...normaliseCapabilities(capsWithoutNotes()),
      notes: ["Memcached cannot enumerate keys"],
    };
    expect(normaliseCapabilities(withNotes).notes).toEqual([
      "Memcached cannot enumerate keys",
    ]);
  });

  it("tolerates a completely undefined object", () => {
    const caps = normaliseCapabilities(undefined);
    expect(caps.notes).toEqual([]);
    expect(caps.kind).toBe("redis");
  });
});

describe("normaliseContainer", () => {
  it("supplies empty meta when omitted", () => {
    const container = { id: "db:0", name: "DB0", kind: "redis_db" } as Container;
    const normalised = normaliseContainer(container);
    expect(normalised.meta).toEqual({});
    // The expression used in the JSX title attribute.
    expect(() => Object.entries(normalised.meta)).not.toThrow();
  });

  it("keeps meta that is present", () => {
    const container = {
      id: "orders",
      name: "orders",
      kind: "kafka_topic",
      meta: { partitions: "3" },
    } as Container;
    expect(normaliseContainer(container).meta).toEqual({ partitions: "3" });
  });
});

describe("normaliseEntry", () => {
  it("supplies empty meta when omitted", () => {
    const entry = { id: "k", label: "k", kind: "string" } as Entry;
    const normalised = normaliseEntry(entry);
    expect(normalised.meta).toEqual({});
    expect(normalised.meta.preview).toBeUndefined();
  });

  it("does not invent a payload", () => {
    const entry = { id: "k", label: "k", kind: "string" } as Entry;
    // A list row legitimately has no payload; the detail pane refetches it.
    expect(normaliseEntry(entry).payload).toBeUndefined();
  });
});

describe("normaliseEntryPage", () => {
  it("supplies an entries array when omitted", () => {
    const page = normaliseEntryPage({} as never);
    expect(page.entries).toEqual([]);
  });

  it("normalises every entry in the page", () => {
    const page = normaliseEntryPage({
      entries: [{ id: "a", label: "a", kind: "string" }],
    } as never);
    expect(page.entries[0]!.meta).toEqual({});
  });

  it("tolerates undefined", () => {
    expect(normaliseEntryPage(undefined).entries).toEqual([]);
  });
});

describe("normaliseSession", () => {
  it("repairs a session whose capabilities omit fields", () => {
    const session = {
      session_id: "s1",
      name: "local redis",
      kind: "redis",
      endpoints: ["127.0.0.1:6379"],
      info: {
        kind: "redis",
        capabilities: capsWithoutNotes(),
      },
    } as unknown as SessionDescriptor;

    const normalised = normaliseSession(session);
    expect(normalised.info.capabilities.notes).toEqual([]);
    expect(normalised.endpoints).toEqual(["127.0.0.1:6379"]);
  });

  it("supplies endpoints when omitted", () => {
    const session = {
      session_id: "s1",
      name: "x",
      kind: "redis",
      info: { kind: "redis", capabilities: capsWithoutNotes() },
    } as unknown as SessionDescriptor;
    expect(normaliseSession(session).endpoints).toEqual([]);
  });
});
