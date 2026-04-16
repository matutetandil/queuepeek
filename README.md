# queuepeek

A terminal UI for inspecting and managing message queues across RabbitMQ, Kafka, and MQTT.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

Browse queues, read messages, publish, diff, dump, replay, and manage your brokers — all from the terminal. Wizard-style drill-down flow: **Profiles -> Queues -> Messages -> Detail**.

## Highlights

- **Multi-broker** — RabbitMQ (Management API), Kafka (librdkafka), MQTT (rumqttc)
- **Non-destructive peek** — RabbitMQ uses ack_requeue_true; Kafka uses ephemeral consumers
- **Message operations** — publish, copy, move, delete, export, import, diff, replay
- **Multi-select** — bulk operations on selected messages with visual checkboxes
- **Smart decode** — JSON/XML pretty-print, base64/gzip auto-decode, Avro via Schema Registry, Protobuf raw decode
- **Queue management** — purge, delete, compare queues, topology view, consumer groups
- **Monitoring** — sparklines, auto-refresh, tail mode, webhook alerts with regex matching
- **Benchmarking** — concurrent flood-publish with p50/p95/p99 latency percentiles
- **DLQ workflows** — x-death header parsing, one-click re-route to original exchange
- **5 color themes** — with live preview picker
- **Scheduling** — delayed message publishing with persistent timers

## Install

### Pre-built binaries

Download from [GitHub Releases](https://github.com/matutetandil/queuepeek/releases) — available for macOS (ARM/Intel), Linux (x86), and Windows (x86/ARM). For Linux ARM, install via `cargo install` or build from source.

### From crates.io

```bash
# Requires cmake for librdkafka
cargo install queuepeek
```

### From source

```bash
git clone https://github.com/matutetandil/queuepeek.git
cd queuepeek && cargo build --release
./target/release/queuepeek
```

### Updating

queuepeek checks for updates automatically. When a new version is available, a hint appears in the status bar — press `Shift+U` to update in place.

## Configuration

```toml
# ~/.config/queuepeek/config.toml

[profiles.local]
type     = "rabbitmq"
host     = "localhost"
port     = 15672
username = "guest"
password = "guest"
vhost    = "/"

[profiles.kafka-dev]
type     = "kafka"
host     = "localhost"
port     = 9092

[profiles.kafka-dev.schema_registry]
url      = "http://localhost:8081"
```

See the [full configuration guide](docs/configuration.md) for TLS, MQTT topics, webhook alerts, templates, and more.

## Keyboard shortcuts

| Key | Queue list | Message list | Detail |
|-----|-----------|-------------|--------|
| `j/k` | Navigate | Navigate | Scroll |
| `Enter` | Open queue | Open message | — |
| `/` | Filter | Filter | Search payload |
| `Shift+P` | Publish | Publish | — |
| `p` | — | — | Pretty-print |
| `b` | — | — | Base64/gzip decode |
| `s` | — | — | Schema Registry decode |
| `e`/`E` | — | Export / pretty | — |
| `Space` | — | Toggle select | — |
| `i` | Queue info | — | — |
| `Shift+G` | Consumer groups | — | — |
| `Shift+X` | Exchanges | — | — |
| `F5` | Benchmark | — | — |
| `n`/`N` | — | — | Next/prev match |
| `?` | Help | Help | Help |
| `Esc` | Back | Back/clear | Back |

See the [full keyboard reference](docs/keyboard-shortcuts.md) for all shortcuts.

## Documentation

- [Configuration](docs/configuration.md) — profiles, TLS, Schema Registry, webhooks, templates
- [Keyboard shortcuts](docs/keyboard-shortcuts.md) — complete key reference per screen
- [Backends](docs/backends.md) — RabbitMQ, Kafka, MQTT details and operation matrix
- [Features](docs/features.md) — deep dive into every feature
- [Architecture](docs/architecture.md) — codebase structure and design patterns

## Tech stack

[ratatui](https://github.com/ratatui-org/ratatui) | [crossterm](https://github.com/crossterm-rs/crossterm) | [rdkafka](https://github.com/fede1024/rust-rdkafka) | [rumqttc](https://github.com/bytebeamio/rumqtt) | [reqwest](https://github.com/seanmonstar/reqwest) | [apache-avro](https://github.com/apache/avro)

## License

MIT
