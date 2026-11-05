import { describe, expect, it } from "vitest";

import {
  BACKEND_LABELS,
  base64ToBytes,
  bytesToBase64,
  formatBytes,
  formatDuration,
  formatTtl,
  hexPreview,
  looksLikeJson,
  payloadText,
  prettyJson,
  truncate,
} from "@/lib/format";

describe("formatBytes", () => {
  it("shows raw bytes below 1 KB", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("scales through units with sensible precision", () => {
    expect(formatBytes(1024)).toBe("1.00 KB");
    expect(formatBytes(1536)).toBe("1.50 KB");
    expect(formatBytes(10 * 1024)).toBe("10.0 KB");
    expect(formatBytes(512 * 1024 * 1024)).toBe("512 MB");
  });

  it("renders a dash for unknown sizes", () => {
    expect(formatBytes(null)).toBe("—");
    expect(formatBytes(undefined)).toBe("—");
  });
});

describe("formatTtl", () => {
  it("maps the sentinel values a server can return", () => {
    expect(formatTtl(-1)).toBe("persistent");
    expect(formatTtl(-2)).toBe("missing");
    expect(formatTtl(null)).toBe("—");
  });

  it("renders compound durations", () => {
    expect(formatTtl(45)).toBe("45s");
    expect(formatTtl(90)).toBe("1m 30s");
    expect(formatTtl(3600)).toBe("1h");
    expect(formatTtl(3700)).toBe("1h 1m");
    expect(formatTtl(86400)).toBe("1d");
    expect(formatTtl(90000)).toBe("1d 1h");
  });
});

describe("formatDuration", () => {
  it("switches to seconds past a second", () => {
    expect(formatDuration(12.4)).toBe("12 ms");
    expect(formatDuration(2500)).toBe("2.50 s");
  });
});

describe("truncate", () => {
  it("collapses whitespace and elides long values", () => {
    expect(truncate("a\n\n  b")).toBe("a b");
    expect(truncate("x".repeat(40), 10)).toHaveLength(10);
    expect(truncate("x".repeat(40), 10).endsWith("…")).toBe(true);
  });
});

describe("payloadText", () => {
  it("prefers structured JSON", () => {
    const text = payloadText({
      json: { a: 1 },
      text: '{"a":1}',
      base64: "",
      binary: false,
    });
    expect(text).toContain('"a": 1');
  });

  it("falls back to text", () => {
    expect(
      payloadText({ text: "hello", base64: "", binary: false }),
    ).toBe("hello");
  });

  it("hex dumps binary payloads", () => {
    const text = payloadText({
      text: null,
      base64: bytesToBase64(new Uint8Array([0x00, 0xff, 0x10])),
      binary: true,
    });
    expect(text).toBe("00 ff 10");
  });
});

describe("hexPreview", () => {
  it("marks truncated previews", () => {
    const bytes = new Uint8Array(200).fill(0xab);
    const preview = hexPreview(bytesToBase64(bytes), 4);
    expect(preview).toBe("ab ab ab ab …");
  });

  it("handles empty payloads", () => {
    expect(hexPreview("")).toBe("(empty)");
  });
});

describe("base64 round trip", () => {
  it("survives arbitrary bytes", () => {
    const original = new Uint8Array([0, 1, 2, 127, 128, 254, 255]);
    expect(Array.from(base64ToBytes(bytesToBase64(original)))).toEqual(
      Array.from(original),
    );
  });
});

describe("json helpers", () => {
  it("detects valid JSON objects and arrays", () => {
    expect(looksLikeJson('{"a":1}')).toBe(true);
    expect(looksLikeJson("[1,2]")).toBe(true);
    expect(looksLikeJson("plain text")).toBe(false);
    expect(looksLikeJson("{broken")).toBe(false);
  });

  it("pretty prints only when parseable", () => {
    expect(prettyJson('{"a":1}')).toBe('{\n  "a": 1\n}');
    expect(prettyJson("{}broken")).toBe("{}broken");
    expect(prettyJson("not json")).toBe("not json");
  });
});

describe("backend labels", () => {
  it("covers every backend the UI can show", () => {
    for (const kind of [
      "redis",
      "memcached",
      "kafka",
      "rabbitmq",
      "nats",
      "mqtt",
    ]) {
      expect(BACKEND_LABELS[kind]).toBeTruthy();
    }
  });
});
