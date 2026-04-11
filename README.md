# queuepeek

A terminal UI for inspecting and managing message queues, built with Rust and ratatui.

## Overview

queuepeek connects to message broker management APIs and lets you browse queues/topics, read messages, publish messages, and perform queue management operations from the terminal. The interface follows a wizard-style drill-down flow: select a profile, pick a queue, browse messages, and inspect individual message detail.

## Features

- Wizard-style drill-down navigation: Profiles -> Queues -> Messages -> Message Detail
- Multi-broker support: RabbitMQ, Kafka, and MQTT
- JSON and XML auto-detection with pretty-print toggle
- Clipboard support for copying payload and headers
- 5 built-in color themes with live preview picker
- Auto-refresh queue list every 5 seconds
- Sparklines in queue list showing publish rate history (last 60 data points / ~5 minutes)
- Queue filtering by name
- Message filtering within a queue (simple substring or advanced field-based expressions)
- Saved filters / bookmarks per queue (save and load named filter expressions)
- Queue publish/deliver rate display
- Message position indicator in detail view
- Fetch count picker popup with presets
- Breadcrumb navigation in all screen headers
- Styled keybinding footers on all screens
- TLS support with client certificate authentication
- Multi-profile configuration for managing multiple environments
- Vhost and namespace picker
- Profile form with type selection (rabbitmq / kafka / mqtt)
- Cloud host auto-detection
- Publish messages to a queue or topic with a multi-line body editor
- Payload templates with variable interpolation (save and load named templates)
- Purge all messages from a queue (RabbitMQ and Kafka, with confirmation)
- Delete a queue or topic (RabbitMQ, Kafka) with confirmation
- Copy all messages from one queue to another, preserving order
- Move all messages from one queue to another (destructive)
- Queue picker popup with filter for selecting copy/move destination
- Multi-select messages with visual checkboxes (Space to toggle, 'a' to select/deselect all)
- Copy selected messages to another queue via queue picker popup
- Delete selected messages with confirmation (consume-and-requeue approach)
- Export selected messages to a JSON file in the current directory
- Re-publish selected messages to the same queue (useful for retry workflows)
- Dump entire queue to JSONL file (streaming, low memory, per-backend strategy)
- Import messages from JSONL or JSON file into the current queue
- Message diff side-by-side (`d` with 2 messages selected) using colored line-by-line comparison
- Queue info popup (`i` on queue list) with detailed stats, rates with bar charts, and configuration
- Exchange and binding topology view (`X` on queue list) showing exchanges -> bindings -> queues
- Message list auto-refresh / tail mode (`T` toggle, `r` manual refresh) with `[live ⟳]` indicator
- JSON and XML syntax highlighting in message detail (colored keys, strings, numbers)
- Base64 and gzip auto-decode in message detail (`b` key) with format label showing decode method
- Schema Registry / Avro decode in message detail (`s` key) — Confluent wire format auto-decode
- Edit & re-publish message from detail view (`E` key)
- DLQ detection and re-routing (`L` key) — parses x-death header, re-publishes to original exchange
- Kafka consumer groups popup (`G` on queue list) with per-partition lag info
- Kafka consumer group offset reset (`R` in consumer groups popup)
- Kafka message replay from offset range to destination topic (`Y` key in message list)
- Benchmark / load testing (`F5` on queue list) — concurrent flood-publish with throughput and latency percentiles (p50/p95/p99)
- Stream-based delete uses temp file backup for safe recovery on failure
- Auto-update check on startup and hourly via GitHub Releases
- Message scheduling with in-app timer (`Ctrl+S` in publish popup, `S` to view/cancel pending)
- Scheduled messages persisted to `~/.config/queuepeek/scheduled.json` (survive restarts)
- Queue comparison / diff between two queues (`=` key on queue list)
- MQTT retained message management (`H` on queue list) — scan, view, and clear retained messages
- ACL / permission viewer (`A` on queue list) — color-coded permission table (RabbitMQ)
- Webhook alerts on message pattern match (`W` on queue list) — regex-based HTTP POST notifications

## Supported Backends

| Backend  | Status       | Notes                                                    |
|----------|--------------|----------------------------------------------------------|
| RabbitMQ | Full support | Management HTTP API, non-destructive peek (ack_requeue)  |
| Kafka    | Full support | Topic listing via metadata, consumer-based message fetch |
| MQTT     | Full support | Wildcard topic discovery, subscription-based reading     |

### Backend Details

**RabbitMQ** uses the Management HTTP API. Messages are peeked using `ack_requeue_true`, so the queue state is never modified during inspection. Publish uses the default exchange with the queue name as the routing key. Purge and delete are supported via the management API.

**Kafka** connects via the native protocol using `librdkafka`. Topics are discovered from cluster metadata, and messages are consumed from tail offsets using ephemeral consumer groups with auto-commit disabled. Publishing uses `BaseProducer`. Topic deletion is supported.

**MQTT** connects via the MQTT protocol. Topics are discovered by subscribing to the wildcard topic `#`, or you can pre-configure specific topics in your profile. Note: MQTT subscriptions consume messages — there is no non-destructive peek. A warning is displayed in the UI. Publishing uses QoS 1. Retained messages can be listed and cleared via the `H` key on the queue list.

### Operation Support Matrix

| Operation                        | RabbitMQ | Kafka | MQTT   |
|----------------------------------|----------|-------|--------|
| List queues                      | Yes      | Yes   | Yes    |
| Fetch messages                   | Yes      | Yes   | Yes    |
| Publish                          | Yes      | Yes   | Yes    |
| Purge queue                      | Yes      | Yes   | No     |
| Delete queue                     | Yes      | Yes   | No     |
| Copy messages                    | Yes      | No    | Yes*   |
| Move messages                    | Yes      | No    | Yes*   |
| Multi-select messages            | Yes      | Yes   | Yes    |
| Copy selected messages           | Yes      | No    | Yes    |
| Delete selected                  | Yes      | No    | Yes*   |
| Export selected to JSON          | Yes      | Yes   | Yes    |
| Re-publish selected              | Yes      | Yes   | Yes    |
| Dump queue to JSONL              | Yes      | Yes   | Yes    |
| Import from JSONL/JSON           | Yes      | Yes   | Yes    |
| Compare queues                   | Yes      | Yes   | Yes    |
| Schedule messages                | Yes      | Yes   | Yes    |
| Reset consumer group offsets     | No       | Yes   | No     |
| Topology view                    | Yes      | No    | No     |
| Replay messages                  | No       | Yes   | No     |
| Benchmark / load test            | Yes      | Yes   | Yes    |
| Retained message management      | No       | No    | Yes    |
| ACL / permission viewer          | Yes      | No    | No     |
| Webhook alerts                   | Yes      | Yes   | Yes    |
| Avro / Schema Registry decode    | Yes      | Yes   | No     |

\* MQTT subscriptions are inherently destructive (messages are consumed on read). Copy, move, and delete operations work but read from the subscription stream — there is no non-destructive peek.

## Prerequisites

- Rust 1.70 or newer
- CMake (for building the bundled librdkafka used by the Kafka backend)
- For RabbitMQ: Management Plugin enabled (`rabbitmq-plugins enable rabbitmq_management`)

## Installation

Install with Cargo:

```bash
cargo install queuepeek
```

Or build from source:

```bash
git clone https://github.com/matutedenda/queuepeek.git
cd queuepeek
cargo build --release
./target/release/queuepeek
```

## Configuration

queuepeek reads its configuration from `~/.config/queuepeek/config.toml`.

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

[profiles.production]
type     = "rabbitmq"
host     = "rabbit.internal.company.com"
port     = 15671
username = "admin"
password = "secret"
vhost    = "/app"
tls      = true
tls_cert = "/path/to/client.crt"
tls_key  = "/path/to/client.key"
tls_ca   = "/path/to/ca.crt"
schema_registry = "http://schema-registry.internal.company.com:8081"

[profiles.my-kafka]
type     = "kafka"
host     = "kafka.example.com"
port     = 9092
username = "admin"
password = "secret"
schema_registry = "http://schema-registry.example.com:8081"

[profiles.my-mqtt]
type     = "mqtt"
host     = "mqtt.example.com"
port     = 1883
username = ""
password = ""
topics   = ["sensors/#", "devices/status"]
```

Each `[profiles.<name>]` block defines a named environment. The `type` field determines which backend is used (`rabbitmq`, `kafka`, or `mqtt`). The `default_profile` key sets which profile is selected on startup.

### MQTT Topics

For MQTT profiles, you can optionally specify a `topics` array to pre-configure which topics to monitor. If omitted, queuepeek subscribes to the wildcard topic `#` and discovers topics automatically (limited to a 3-second discovery window).

### TLS

Set `tls = true` to enable TLS. Provide `tls_cert`, `tls_key`, and `tls_ca` paths for mutual TLS with a client certificate.

- **RabbitMQ**: Uses HTTPS for the Management API
- **Kafka**: Uses SASL_SSL with PLAIN mechanism
- **MQTT**: Uses mqtts:// (MQTT over TLS)

### Schema Registry

Set `schema_registry` in a profile to point to a Confluent-compatible Schema Registry URL. When configured, the `s` key in the message detail view will attempt to decode Avro messages using the registry. Messages must use the Confluent wire format (magic byte `0x00` followed by a 4-byte schema ID). Decoded content is rendered as pretty-printed JSON and combines with syntax highlighting.

### Saved Filters

Named filter expressions can be saved per queue using `Ctrl+B` in the message list filter input. Saved filters are stored in `config.toml` and can be loaded at any time with `B`. This is useful for recurring search patterns (e.g. filtering by a specific routing key or header value).

### Payload Templates

Message body templates can be saved and loaded in the publish popup. Use `Ctrl+W` to save the current body as a named template, and `Ctrl+T` to load a saved template. Templates support variable interpolation: `{{timestamp}}`, `{{uuid}}`, `{{random_int}}`, `{{counter}}`, and `{{env.VAR}}`. Variables are resolved at publish time.

### Webhook Alerts

Press `W` on a selected queue in the queue list to open the webhook alert configuration popup. Specify a regex pattern and an HTTP POST endpoint URL. During auto-refresh, if any incoming message body matches the pattern, queuepeek sends a POST request with the message details to the webhook URL. Alerts are deduplicated by content hash within a session. Configuration is persisted in `config.toml`.

### Scheduled Messages

Scheduled messages are saved to `~/.config/queuepeek/scheduled.json` so they persist across restarts. On startup, any messages whose scheduled time has already passed are published immediately. Messages that are still pending resume their countdown from the remaining time.

## Usage

Start the application:

```bash
queuepeek
```

The wizard flow proceeds through four screens:

1. **Profile selection** — Choose a saved profile or create a new one.
2. **Queue list** — Browse queues/topics for the active namespace. Queues auto-refresh every 5 seconds. Filter by name with `/`. Publish, purge, delete, copy, move, compare, schedule, topology, benchmark, retained messages, ACL, and webhook alert operations are available here.
3. **Message list** — Browse fetched messages for the selected queue/topic. Filter messages with `/` (Tab to toggle advanced filter mode). Use Space to select individual messages or `a` to select all. Perform bulk operations on the selection with `C`, `D`, `e`, `R`, `d` (diff), or `Y` (replay).
4. **Message detail** — View full message payload, headers, and metadata. Toggle pretty-print, toggle binary decode (`b`), toggle Avro decode (`s`), copy to clipboard, scroll through the payload.

Press `Esc` or `Backspace` at any screen to go back one level. In the message list, the first `Esc` clears the current selection; the second `Esc` returns to the queue list.

## Keyboard Shortcuts

### Profile Screen

| Key          | Action                        |
|--------------|-------------------------------|
| `j` / Down   | Move selection down           |
| `k` / Up     | Move selection up             |
| `Enter`      | Connect with selected profile |
| `a`          | Create new profile            |
| `e`          | Edit selected profile         |
| `d`          | Delete selected profile       |
| `q` / Ctrl+C | Quit                          |

### Queue List Screen

| Key          | Action                                               |
|--------------|------------------------------------------------------|
| `j` / Down   | Move selection down                                  |
| `k` / Up     | Move selection up                                    |
| `Enter`      | Open selected queue                                  |
| `/`          | Filter queues by name                                |
| `r`          | Refresh queue list                                   |
| `f`          | Open fetch count picker                              |
| `+` / `-`   | Adjust fetch count by 10                             |
| `v`          | Switch namespace / vhost                             |
| `p`          | Switch profile                                       |
| `P`          | Publish a message to the selected queue              |
| `x`          | Purge selected queue (with confirmation)             |
| `D`          | Delete selected queue/topic (with confirmation)      |
| `C`          | Copy all messages to another queue (with picker)     |
| `m`          | Move all messages to another queue (with picker)     |
| `=`          | Compare selected queue with another queue (diff)     |
| `i`          | Show detailed queue/topic info (stats, config, rates)|
| `G`          | Show consumer groups for selected topic (Kafka)      |
| `X`          | Show exchange and binding topology (RabbitMQ)        |
| `F5`         | Benchmark: flood-publish N messages, show throughput |
| `S`          | View pending scheduled messages                      |
| `H`          | Manage retained messages (MQTT only)                 |
| `A`          | View ACL / permission table (RabbitMQ only)          |
| `W`          | Configure webhook alert for selected queue           |
| `t`          | Open theme picker                                    |
| `Esc`        | Go back to profile screen                            |
| `q` / Ctrl+C | Quit                                                 |

### Queue Picker Popup (Copy / Move / Compare destination)

| Key        | Action                          |
|------------|---------------------------------|
| `j` / Down | Move selection down             |
| `k` / Up   | Move selection up               |
| `/`        | Filter queues by name           |
| `Enter`    | Confirm destination queue       |
| `Esc`      | Cancel                          |

### Publish Message Popup

| Key         | Action                                |
|-------------|---------------------------------------|
| `Tab`       | Move to next field                    |
| `Shift+Tab` | Move to previous field                |
| `Enter`     | Send message (in body field: newline) |
| `Ctrl+Enter`| Send message                          |
| `Ctrl+S`    | Schedule message with delay           |
| `Ctrl+T`    | Load a saved payload template         |
| `Ctrl+W`    | Save current body as a named template |
| `Esc`       | Cancel                                |

### Message List Screen

| Key          | Action                                                          |
|--------------|-----------------------------------------------------------------|
| `j` / Down   | Move selection down                                             |
| `k` / Up     | Move selection up                                               |
| `Enter`      | Open message detail                                             |
| `/`          | Filter messages                                                 |
| `Tab`        | Toggle between simple and advanced filter mode (in filter input)|
| `B`          | Load a saved filter / bookmark                                  |
| `Ctrl+B`     | Save current filter as a named bookmark                         |
| `Space`      | Toggle selection on the current message (shows checkbox)        |
| `a`          | Select all messages / deselect all if all are selected          |
| `f`          | Open fetch count picker                                         |
| `+` / `-`   | Adjust fetch count by 10                                        |
| `r`          | Re-fetch messages                                               |
| `C`          | Copy selected messages to another queue (queue picker popup)    |
| `M`          | Move selected messages to another queue                         |
| `D`          | Delete selected messages (destructive, with confirmation)       |
| `d`          | Diff two selected messages side-by-side (requires exactly 2)   |
| `e`          | Export selected messages to a JSON file in the current directory|
| `R`          | Re-publish selected messages to the same queue                  |
| `W`          | Dump entire queue to JSONL file (streaming, per-backend strategy)|
| `I`          | Import messages from a JSONL or JSON file                       |
| `L`          | DLQ re-route: re-publish to original exchange (x-death header)  |
| `T`          | Toggle auto-refresh / tail mode (every 5 seconds)               |
| `Y`          | Replay messages from offset range to destination topic (Kafka)  |
| `S`          | View pending scheduled messages                                 |
| `P`          | Publish a new message to the current queue                      |
| `Esc`        | Clear selection (first press) / go back to queue list (second)  |
| `q` / Ctrl+C | Quit                                                            |

Selection state is shown as a checkbox prefix on each message row (☑ selected, ☐ unselected). The number of selected messages is displayed in the screen header and in the footer when at least one message is selected.

### Message Detail Screen

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / Down   | Scroll payload down             |
| `k` / Up     | Scroll payload up               |
| PgDown/PgUp  | Scroll 10 lines at a time       |
| `p`          | Toggle pretty-print             |
| `b`          | Toggle base64/gzip decode       |
| `s`          | Toggle Avro / Schema Registry decode |
| `c`          | Copy payload to clipboard       |
| `h`          | Copy headers to clipboard       |
| `E`          | Edit & re-publish message       |
| `L`          | DLQ re-route (x-death header)   |
| `Esc`        | Go back to message list         |
| `q` / Ctrl+C | Quit                            |

### Consumer Groups Popup (Kafka)

| Key   | Action                                      |
|-------|---------------------------------------------|
| `j`   | Move selection down                         |
| `k`   | Move selection up                           |
| `R`   | Reset offsets for the selected group        |
| `Esc` | Close popup                                 |

### Scheduled Messages Popup

| Key   | Action                                      |
|-------|---------------------------------------------|
| `j`   | Move selection down                         |
| `k`   | Move selection up                           |
| `d`   | Cancel the selected scheduled message       |
| `Esc` | Close popup                                 |

### Queue Comparison Results Popup

| Key         | Action                           |
|-------------|----------------------------------|
| `Tab`       | Switch to next tab               |
| `Shift+Tab` | Switch to previous tab           |
| `j` / Down  | Scroll down                      |
| `k` / Up    | Scroll up                        |
| `Esc`       | Close popup                      |

### Topology View Popup (RabbitMQ)

| Key   | Action          |
|-------|-----------------|
| `j`   | Scroll down     |
| `k`   | Scroll up       |
| `Esc` | Close popup     |

### Retained Messages Popup (MQTT)

| Key   | Action                                   |
|-------|------------------------------------------|
| `j`   | Move selection down                      |
| `k`   | Move selection up                        |
| `d`   | Clear the selected retained message      |
| `Esc` | Close popup                              |

### Replay Config Popup (Kafka)

| Key         | Action                          |
|-------------|---------------------------------|
| `Tab`       | Move to next field              |
| `Shift+Tab` | Move to previous field          |
| `Enter`     | Start replay                    |
| `Esc`       | Cancel                          |

### Benchmark Popup

| Key         | Action                          |
|-------------|---------------------------------|
| `Tab`       | Move to next field              |
| `Shift+Tab` | Move to previous field          |
| `Enter`     | Start benchmark                 |
| `Esc`       | Cancel / stop in-progress run   |

## Architecture

The application is organized into focused modules:

```
src/
├── main.rs              # Entry point (~98 lines): terminal init, event loop
├── app.rs               # Central state machine: Screen enum, Popup variants,
│                        # BgResult variants, background task launchers
├── config.rs            # Config structs: Profile, SavedFilter, MessageTemplate,
│                        # WebhookAlert; load/save for config.toml and scheduled.json
├── updater.rs           # Auto-update via GitHub Releases
├── filters.rs           # Filter expression parser and evaluator
├── comparison.rs        # Queue diff / comparison logic
├── operations.rs        # Background operation helpers (copy, move, delete, dump, etc.)
├── utils.rs             # Shared utilities
├── keys/                # Key handlers, one module per screen
│   ├── profiles.rs
│   ├── queue_list.rs
│   ├── message_list.rs
│   ├── message_detail.rs
│   └── popups.rs
├── backend/
│   ├── mod.rs           # Backend trait and shared structs
│   ├── rabbitmq.rs      # RabbitMQ Management API implementation
│   ├── kafka.rs         # Kafka implementation (rdkafka)
│   └── mqtt.rs          # MQTT implementation (rumqttc)
└── ui/
    ├── mod.rs            # Draw dispatcher
    ├── profiles.rs       # Profile list and form rendering
    ├── queue_list.rs     # Queue list rendering with sparklines
    ├── message_list.rs   # Message list rendering with multi-select
    ├── message_detail.rs # Message detail with syntax highlighting and decode
    ├── popup.rs          # All popup rendering functions
    └── theme.rs          # 5 built-in themes with live preview
```

The `Backend` trait abstracts all broker-specific logic. Background I/O runs in threads that communicate back to the main event loop via `std::sync::mpsc` channels. The event loop polls both crossterm input events and the mpsc receiver on each tick, keeping the UI responsive.

## Tech Stack

- [ratatui](https://github.com/ratatui-org/ratatui) — Terminal UI rendering
- [crossterm](https://github.com/crossterm-rs/crossterm) — Cross-platform terminal control
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client for RabbitMQ Management API and webhook delivery
- [rdkafka](https://github.com/fede1024/rust-rdkafka) — Kafka client (librdkafka wrapper)
- [rumqttc](https://github.com/bytebeamio/rumqtt) — MQTT client
- [serde / toml](https://github.com/toml-rs/toml) — Configuration parsing
- [arboard](https://github.com/1Password/arboard) — Clipboard support
- [self_update](https://github.com/jaemk/self_update) — Auto-update from GitHub Releases
- [similar](https://github.com/mitsuhiko/similar) — Diff engine for message comparison
- [base64](https://github.com/marshallpierce/rust-base64) — Base64 encoding/decoding
- [flate2](https://github.com/rust-lang/flate2-rs) — Gzip decompression
- [regex](https://github.com/rust-lang/regex) — Regex pattern matching for webhook alerts
- [apache-avro](https://github.com/apache/avro) — Avro decoding for Schema Registry integration

## License

MIT
