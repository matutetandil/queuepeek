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
- Queue filtering by name
- Message filtering within a queue
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
- Purge all messages from a queue (RabbitMQ) with confirmation
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
- Stream-based delete uses temp file backup for safe recovery on failure
- Auto-update check on startup and hourly via GitHub Releases

## Supported Backends

| Backend  | Status       | Notes                                                    |
|----------|--------------|----------------------------------------------------------|
| RabbitMQ | Full support | Management HTTP API, non-destructive peek (ack_requeue)  |
| Kafka    | Full support | Topic listing via metadata, consumer-based message fetch |
| MQTT     | Full support | Wildcard topic discovery, subscription-based reading     |

### Backend Details

**RabbitMQ** uses the Management HTTP API. Messages are peeked using `ack_requeue_true`, so the queue state is never modified during inspection. Publish uses the default exchange with the queue name as the routing key. Purge and delete are supported via the management API.

**Kafka** connects via the native protocol using `librdkafka`. Topics are discovered from cluster metadata, and messages are consumed from tail offsets using ephemeral consumer groups with auto-commit disabled. Publishing uses `BaseProducer`. Topic deletion is supported.

**MQTT** connects via the MQTT protocol. Topics are discovered by subscribing to the wildcard topic `#`, or you can pre-configure specific topics in your profile. Note: MQTT subscriptions consume messages — there is no non-destructive peek. A warning is displayed in the UI. Publishing uses QoS 1.

### Operation Support Matrix

| Operation              | RabbitMQ | Kafka | MQTT   |
|------------------------|----------|-------|--------|
| List queues            | Yes      | Yes   | Yes    |
| Fetch messages         | Yes      | Yes   | Yes    |
| Publish                | Yes      | Yes   | Yes    |
| Purge queue            | Yes      | Yes   | No     |
| Delete queue           | Yes      | Yes   | No     |
| Copy messages          | Yes      | No    | Yes*   |
| Move messages          | Yes      | No    | Yes*   |
| Multi-select messages  | Yes      | Yes   | Yes    |
| Copy selected messages | Yes      | No    | Yes    |
| Delete selected        | Yes      | No    | Yes*   |
| Export selected to JSON| Yes      | Yes   | Yes    |
| Re-publish selected    | Yes      | Yes   | Yes    |
| Dump queue to JSONL    | Yes      | Yes   | Yes    |
| Import from JSONL/JSON | Yes      | Yes   | Yes    |

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

[profiles.my-kafka]
type     = "kafka"
host     = "kafka.example.com"
port     = 9092
username = "admin"
password = "secret"

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

## Usage

Start the application:

```bash
queuepeek
```

The wizard flow proceeds through four screens:

1. **Profile selection** — Choose a saved profile or create a new one.
2. **Queue list** — Browse queues/topics for the active namespace. Queues auto-refresh every 5 seconds. Filter by name with `/`. Publish, purge, delete, copy, and move operations are available here.
3. **Message list** — Browse fetched messages for the selected queue/topic. Filter messages with `/`. Use Space to select individual messages or `a` to select all. Perform bulk operations on the selection with `C`, `D`, `e`, or `R`.
4. **Message detail** — View full message payload, headers, and metadata. Toggle pretty-print, copy to clipboard, scroll through the payload.

Press `Esc` or `Backspace` at any screen to go back one level. In the message list, the first `Esc` clears the current selection; the second `Esc` returns to the queue list.

## Keyboard Shortcuts

### Profile Screen

| Key          | Action                        |
|--------------|-------------------------------|
| `j` / Down   | Move selection down           |
| `k` / Up     | Move selection up             |
| `Enter`      | Connect with selected profile |
| `n`          | Create new profile            |
| `e`          | Edit selected profile         |
| `d`          | Delete selected profile       |
| `q` / Ctrl+C | Quit                          |

### Queue List Screen

| Key          | Action                                              |
|--------------|-----------------------------------------------------|
| `j` / Down   | Move selection down                                 |
| `k` / Up     | Move selection up                                   |
| `Enter`      | Open selected queue                                 |
| `/`          | Filter queues by name                               |
| `r`          | Refresh queue list                                  |
| `P`          | Publish a message to the selected queue             |
| `x`          | Purge selected queue (RabbitMQ, with confirmation)  |
| `D`          | Delete selected queue/topic (with confirmation)     |
| `C`          | Copy all messages to another queue (with picker)    |
| `m`          | Move all messages to another queue (with picker)    |
| `t`          | Open theme picker                                   |
| `Esc`        | Go back to profile screen                           |
| `q` / Ctrl+C | Quit                                                |

### Queue Picker Popup (Copy / Move destination)

| Key        | Action                          |
|------------|---------------------------------|
| `j` / Down | Move selection down             |
| `k` / Up   | Move selection up               |
| `/`        | Filter queues by name           |
| `Enter`    | Confirm destination queue       |
| `Esc`      | Cancel                          |

### Publish Message Popup

| Key         | Action                 |
|-------------|------------------------|
| `Tab`       | Move to next field     |
| `Shift+Tab` | Move to previous field |
| `Enter`     | Send message           |
| `Esc`       | Cancel                 |

### Message List Screen

| Key          | Action                                                          |
|--------------|-----------------------------------------------------------------|
| `j` / Down   | Move selection down                                             |
| `k` / Up     | Move selection up                                               |
| `Enter`      | Open message detail                                             |
| `/`          | Filter messages                                                 |
| `Space`      | Toggle selection on the current message (shows checkbox)        |
| `a`          | Select all messages / deselect all if all are selected          |
| `n`          | Open fetch count picker                                         |
| `r`          | Re-fetch messages                                               |
| `C`          | Copy selected messages to another queue (queue picker popup)    |
| `D`          | Delete selected messages (destructive, with confirmation)       |
| `e`          | Export selected messages to a JSON file in the current directory|
| `R`          | Re-publish selected messages to the same queue                  |
| `W`          | Dump entire queue to JSONL file (streaming, per-backend strategy)|
| `I`          | Import messages from a JSONL or JSON file                       |
| `P`          | Publish a new message to the current queue                      |
| `Esc`        | Clear selection (first press) / go back to queue list (second)  |
| `q` / Ctrl+C | Quit                                                            |

Selection state is shown as a checkbox prefix on each message row (☑ selected, ☐ unselected). The number of selected messages is displayed in the screen header and in the footer when at least one message is selected.

### Message Detail Screen

| Key          | Action                          |
|--------------|---------------------------------|
| `j` / Down   | Scroll payload down             |
| `k` / Up     | Scroll payload up               |
| `p`          | Toggle pretty-print             |
| `c`          | Copy payload to clipboard       |
| `h`          | Copy headers to clipboard       |
| `[`          | Previous message                |
| `]`          | Next message                    |
| `Esc`        | Go back to message list         |
| `q` / Ctrl+C | Quit                            |

## Tech Stack

- [ratatui](https://github.com/ratatui-org/ratatui) — Terminal UI rendering
- [crossterm](https://github.com/crossterm-rs/crossterm) — Cross-platform terminal control
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client for RabbitMQ Management API
- [rdkafka](https://github.com/fede1024/rust-rdkafka) — Kafka client (librdkafka wrapper)
- [rumqttc](https://github.com/bytebeamio/rumqtt) — MQTT client
- [serde / toml](https://github.com/toml-rs/toml) — Configuration parsing
- [arboard](https://github.com/1Password/arboard) — Clipboard support
- [self_update](https://github.com/jaemk/self_update) — Auto-update from GitHub Releases

## License

MIT
