# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

queuepeek — a TUI application for inspecting message queues from the terminal. It uses a wizard-style drill-down flow (Profiles -> Queues -> Messages -> Message Detail) and supports multiple broker backends. RabbitMQ is fully implemented using the Management HTTP API with `ack_requeue_true` for non-destructive peeking. Kafka uses `rdkafka` (librdkafka) for native protocol access. MQTT uses `rumqttc` for subscription-based reading.

## Build and Run

```bash
cargo build                  # debug build (requires cmake for librdkafka)
cargo run                    # run in debug mode
cargo build --release        # optimized release build
./target/release/queuepeek   # run release binary
```

## Architecture

### Core modules
- `src/main.rs` — Entry point (~98 lines). Terminal setup, event loop, auto-refresh timers.
- `src/app.rs` — Central state machine (~1900 lines). Holds App struct with all state, BgResult enum, background task launchers, process_bg_results.
- `src/config.rs` — Loads/saves `~/.config/queuepeek/config.toml`. Profile, WebhookAlert, SavedFilter, MessageTemplate, SchemaRegistryConfig.

### Key handler modules (`src/keys/`)
- `mod.rs` — Top-level key dispatch (handle_key).
- `profile.rs` — Profile screen: select, add/edit form, delete confirm.
- `queue_list.rs` — Queue list: navigation, filter, queue operations, alert/permission/retained shortcuts.
- `message_list.rs` — Message list: multi-select, filter, bulk operations.
- `message_detail.rs` — Message detail: scroll, pretty-print, decode, clipboard, schema toggle.
- `popup.rs` — All popup key handlers (~910 lines). Unified queue picker and publish/edit handlers.

### Backend modules (`src/backend/`)
- `mod.rs` — Backend trait (~20 methods), data structs (QueueInfo, MessageInfo, PermissionEntry, etc.).
- `rabbitmq.rs` — RabbitMQ Management HTTP API. All operations + permissions + topology.
- `kafka.rs` — rdkafka-based. Topics, consumer groups, replay, offset reset.
- `mqtt.rs` — rumqttc-based. Topic discovery, retained messages, publish.

### UI modules (`src/ui/`)
- `mod.rs` — Draw dispatcher per Screen.
- `profiles.rs`, `queue_list.rs`, `message_list.rs`, `message_detail.rs` — One per screen.
- `popup.rs` — ~38 popup draw functions.
- `theme.rs` — 5 themes with picker and live preview.

### Extracted logic modules
- `src/filters.rs` — Advanced filter engine (FilterExpr, parse, eval).
- `src/comparison.rs` — Queue diff algorithm (hash-based).
- `src/operations.rs` — Dump strategies (RabbitMQ/Kafka/MQTT), message serialization, x-death parsing.
- `src/utils.rs` — Template interpolation, clipboard.
- `src/schema.rs` — Schema Registry client, Avro decode, schema cache.
- `src/updater.rs` — Auto-update via GitHub Releases.

## Key Patterns

- Screen navigation follows a wizard flow. The `App` state machine holds a `Screen` enum with variants for each step. `Esc` pops one level back.
- The `Backend` trait abstracts broker-specific logic. Optional methods return `Err("not supported")` by default.
- Background I/O runs in threads via `std::sync::mpsc` channels. The event loop polls crossterm input and the mpsc receiver each tick.
- Auto-refresh: queue list every 5s, message list when tail mode is on.
- Benchmarks use `std::thread::scope` for real parallelism with shared atomic counters.
- Schema Registry decode uses Confluent wire format (0x00 + 4-byte schema ID + payload).
- Webhook alerts check every 30s with regex pattern matching and hash-based deduplication.
- Scheduled messages persist to `~/.config/queuepeek/scheduled.json` using epoch seconds.

## Config Path

```
~/.config/queuepeek/config.toml
```

Example structure:

```toml
default_profile = "local"

[profiles.local]
type     = "rabbitmq"
host     = "localhost"
port     = 15672
username = "guest"
password = "guest"
vhost    = "/"
tls      = false

[profiles.local.schema_registry]
url      = "http://localhost:8081"
username = ""
password = ""

[[webhook_alerts]]
name        = "error-monitor"
pattern     = "(?i)error|exception"
webhook_url = "https://hooks.example.com/alert"
enabled     = true
queues      = []
```
