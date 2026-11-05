# Cube Studio

[![Tauri](https://img.shields.io/badge/tauri-2-brightgreen.svg?style=flat-square)](https://tauri.app)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg?style=flat-square)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/react-18-blue.svg?style=flat-square)](https://react.dev)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)

A desktop client for caches, queues and event logs. Cube Studio browses and
operates on **Redis, Memcached, Kafka, RabbitMQ, NATS and MQTT** from one window,
including managed instances on AWS, Azure, Google Cloud, Confluent Cloud,
Upstash, CloudAMQP and Redis Cloud.

Built with Tauri 2: a Rust core that speaks each protocol natively, and a React
frontend. The distributed app is a single small binary per platform — there is no
bundled Node runtime, and no JVM or librdkafka underneath.

## Install

Grab the installer for your platform from
[Releases](https://github.com/aarint/cube-studio/releases):

| Platform | File |
| --- | --- |
| macOS (Apple Silicon) | `Cube Studio_<version>_aarch64.dmg` |
| macOS (Intel) | `Cube Studio_<version>_x64.dmg` |
| Linux | `cube-studio_<version>_amd64.AppImage` or `.deb` |
| Windows | `Cube Studio_<version>_x64_en-US.msi` or `-setup.exe` |

macOS builds are not code-signed, so Gatekeeper will warn on first launch:
right-click the app → **Open**, or approve it in
**System Settings → Privacy & Security**.

## Features

### One interface, six protocols

Every backend is presented the same way — **containers** on the left (databases,
topics, queues, streams, topic filters), **entries** in the middle, the selected
value on the right — while still exposing what each protocol actually does.

| Backend | Browse | Write | Live tail | Console | Notes |
| --- | --- | --- | --- | --- | --- |
| Redis | databases → keys | all value types + TTL | pub/sub, keyspace events | full command console | standalone, cluster, sentinel |
| Memcached | keys via server stats | set, delete, touch | — | ASCII protocol | no native key enumeration |
| Kafka | topics → messages | publish | per-partition | — | append-only |
| RabbitMQ | queues → messages | publish via exchange | consumer | queue admin verbs | peek never consumes |
| NATS | JetStream streams | publish | subject wildcards | request/reply | core NATS has no history |
| MQTT | retained messages | publish, retain | topic filters | publish | QoS 0/1/2 |

### Cloud discovery

Point Cube Studio at a cloud API key and it lists your managed instances, then
connects using the same drivers as self-hosted servers:

| Provider | Services |
| --- | --- |
| AWS | ElastiCache (incl. Serverless), MSK, Amazon MQ |
| Azure | Azure Cache for Redis |
| Google Cloud | Memorystore for Redis, Memorystore Cluster |
| Confluent Cloud | Kafka |
| Upstash | Redis, Kafka |
| CloudAMQP | RabbitMQ |
| Redis Cloud | Redis |

Discovery is read-only. Cloud credentials are used for the API call and never
written to disk; the data-plane password is stored separately, encrypted.

### Nine languages

简体中文 · 繁體中文 · English · Español · Português · Français · Deutsch ·
日本語 · 한국어

The interface language follows the OS on first launch and can be changed from the
picker in the sidebar.

### Security

- Saved profiles and secrets are encrypted with AES-256-GCM.
- The key is derived from a per-installation random salt bound to the OS user
  account, so copying the data directory to another machine reveals nothing.
- Passwords never appear in logs or in the profile file.
- TLS is supported for every backend, with an optional custom CA bundle.
  "Skip verification" exists but is labelled and warned about inline.

## Development

Requirements: Node 20+, Rust 1.82+, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```bash
npm install
npm run dev            # app with hot reload
```

Useful commands:

```bash
npm run dev:web        # frontend only, in a browser, with a mock backend
npm run typecheck      # TypeScript
npm test               # frontend unit tests (vitest)
npm run test:rust      # Rust unit tests
```

### Integration tests

The Rust tests in `src-tauri/tests/integration.rs` run against real middleware in
Docker and are behind a feature flag so the default suite stays fast:

```bash
docker compose -f docker-compose.test.yml up -d
npm run test:integration
```

### Project layout

```
src/                    React frontend
├── components/         App shell, connection dialog, workspace panes
├── i18n/               One catalogue per language, typed against English
├── lib/                IPC client, preview backend, formatting
└── styles/             Design tokens and component styles

src-tauri/src/          Rust core
├── backend/            One driver per protocol, behind a shared trait
├── cloud/              Cloud provider discovery + SigV4 + HTTP client
├── commands.rs         Tauri IPC surface
├── session.rs          Live connection registry
└── store.rs            Encrypted profile persistence
```

Adding a protocol means implementing the `Backend` trait in `src-tauri/src/backend/`
and adding one arm to the factory there; the UI adapts from the capabilities the
driver reports.

## Releasing

Versions live in three manifests, so bump them together:

```bash
npm run version:set 2.1.0
git commit -am "Release 2.1.0"
git tag v2.1.0
git push origin HEAD --tags
```

Pushing the tag runs `.github/workflows/release.yml`, which builds installers for
macOS (both architectures), Linux and Windows, attaches them to a GitHub release,
then verifies every platform is present and non-empty before finishing.

To build installers locally:

```bash
npm run build                                  # current platform
npx tauri build --target x86_64-apple-darwin   # a specific target
```

Bundles land in `src-tauri/target/<target>/release/bundle/`.

## Contributing

Issues and pull requests are welcome. Please run `npm run typecheck`,
`npm test` and `npm run test:rust` before opening a PR.

## License

[MIT](LICENSE)
