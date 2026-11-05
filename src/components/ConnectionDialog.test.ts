import { describe, expect, it } from "vitest";

import { emptyConfig, validate } from "@/components/ConnectionDialog";
import type { ConnectionConfig } from "@/types/api";

function withChanges(changes: Partial<ConnectionConfig>): ConnectionConfig {
  return { ...emptyConfig(), ...changes };
}

describe("emptyConfig", () => {
  it("uses the canonical port for each backend", () => {
    expect(emptyConfig("redis").port).toBe(6379);
    expect(emptyConfig("memcached").port).toBe(11211);
    expect(emptyConfig("kafka").port).toBe(9092);
    expect(emptyConfig("rabbitmq").port).toBe(5672);
    expect(emptyConfig("nats").port).toBe(4222);
    expect(emptyConfig("mqtt").port).toBe(1883);
  });

  it("starts with TLS disabled and no saved id", () => {
    const config = emptyConfig("redis");
    expect(config.tls).toBe("disabled");
    expect(config.id).toBe("");
  });
});

describe("validate", () => {
  it("accepts a minimal valid profile", () => {
    expect(validate(emptyConfig("redis"))).toBeNull();
    expect(validate(emptyConfig("mqtt"))).toBeNull();
  });

  it("rejects an empty host", () => {
    expect(validate(withChanges({ host: "   " }))).toContain("Host");
  });

  it("rejects out-of-range ports", () => {
    expect(validate(withChanges({ port: 0 }))).toContain("Port");
    expect(validate(withChanges({ port: 70000 }))).toContain("Port");
    expect(validate(withChanges({ port: 1.5 }))).toContain("Port");
  });

  it("requires a sentinel master name", () => {
    expect(
      validate(
        withChanges({ kind: "redis", redis_mode: "sentinel", port: 26379 }),
      ),
    ).toContain("master name");
  });

  it("requires a username for Kafka SASL", () => {
    expect(
      validate(
        withChanges({ kind: "kafka", kafka_sasl_mechanism: "plain", port: 9092 }),
      ),
    ).toContain("username");
  });

  it("allows Kafka SASL with a username", () => {
    expect(
      validate(
        withChanges({
          kind: "kafka",
          kafka_sasl_mechanism: "plain",
          username: "svc",
        }),
      ),
    ).toBeNull();
  });
});
